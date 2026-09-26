//! Schema-directed typed records (G147, ADR 0063).
//!
//! A typed value (a `SemanticObservation`, an `Evidence`, an `ExtractionDiagnostic`) is written
//! field by field through its declared record kind (`schema::SECTIONS`), never as text. A value's
//! serde form is walked against the declaration:
//! - an object's keys are the record's field names; a key the kind does not declare is refused
//!   (a typed field the schema has not caught up with is never silently dropped);
//! - `null` is an absent optional field, and a missing required field is refused;
//! - strings are string-table indices, unsigned integers uvarints, booleans `BOOL`;
//! - a nested struct is one embedded record (`RECORD`), a sequence of structs several, and a
//!   sequence of strings a `PACKED` list of indices; an empty sequence is an absent field;
//! - an enum kind (first field `variant`) carries a unit variant as its name, and a variant with
//!   data as its name plus the field named after it.
//!
//! Decoding rebuilds the same serde form, so `decode(encode(v))` deserializes to `v`.

use super::schema::{self, FieldDef, RecordDef};
use super::{
    AtlasError, ParsedRecord, Record, Strings, WIRE_BOOL, WIRE_LOCAL_INDEX, WIRE_PACKED,
    WIRE_RECORD, WIRE_UVARINT, parse_records, read_uvarint, reject, uvarint,
};
use serde_json::{Map, Value};

fn declared(section: u16, kind: u16) -> Result<&'static RecordDef, String> {
    schema::record(section, kind).ok_or(format!("section {section}: no record kind {kind}"))
}

fn is_enum(def: &RecordDef) -> bool {
    def.fields.first().is_some_and(|f| f.name == "variant")
}

fn field_named(def: &RecordDef, name: &str) -> Option<FieldDef> {
    def.fields.iter().copied().find(|f| f.name == name)
}

/// The fields of `value` present under `def`, in declaration order.
fn present(def: &RecordDef, value: &Value) -> Result<Vec<(FieldDef, Value)>, String> {
    let mut out: Vec<(FieldDef, Value)> = Vec::new();
    if is_enum(def) {
        let variant = def.fields[0];
        match value {
            Value::String(name) => out.push((variant, Value::String(name.clone()))),
            Value::Object(map) if map.len() == 1 => {
                let (name, payload) = map.iter().next().expect("one entry");
                let field = field_named(def, name)
                    .filter(|f| f.name != "variant")
                    .ok_or(format!("{}: undeclared variant `{name}`", def.name))?;
                out.push((variant, Value::String(name.clone())));
                out.push((field, payload.clone()));
            }
            _ => return Err(format!("{}: not an enum value", def.name)),
        }
        return Ok(out);
    }
    let Value::Object(map) = value else {
        return Err(format!("{}: not an object", def.name));
    };
    for (name, item) in map {
        let field =
            field_named(def, name).ok_or(format!("{}: undeclared field `{name}`", def.name))?;
        let empty = item.is_null() || item.as_array().is_some_and(Vec::is_empty);
        if item.is_null() && field.required {
            return Err(format!("{}: required field `{name}` is null", def.name));
        }
        if empty && !field.required {
            continue;
        }
        out.push((field, item.clone()));
    }
    for field in def.fields {
        if field.required && !out.iter().any(|(f, _)| f.tag == field.tag) {
            return Err(format!(
                "{}: missing required field `{}`",
                def.name, field.name
            ));
        }
    }
    out.sort_by_key(|(f, _)| f.tag);
    Ok(out)
}

fn text<'v>(def: &RecordDef, field: &FieldDef, value: &'v Value) -> Result<&'v str, String> {
    value
        .as_str()
        .ok_or(format!("{}.{}: not a string", def.name, field.name))
}

fn items<'v>(def: &RecordDef, field: &FieldDef, value: &'v Value) -> Result<&'v [Value], String> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or(format!("{}.{}: not a sequence", def.name, field.name))
}

/// Every string `value` puts in the string table when written as `kind` of `section`.
pub(super) fn collect_strings(
    section: u16,
    kind: u16,
    value: &Value,
    out: &mut Vec<String>,
) -> Result<(), String> {
    let def = declared(section, kind)?;
    for (field, item) in present(def, value)? {
        match field.wire {
            WIRE_LOCAL_INDEX => out.push(text(def, &field, &item)?.to_owned()),
            WIRE_PACKED => {
                for element in items(def, &field, &item)? {
                    out.push(text(def, &field, element)?.to_owned());
                }
            }
            WIRE_RECORD if field.repeated => {
                for element in items(def, &field, &item)? {
                    collect_strings(section, field.nested, element, out)?;
                }
            }
            WIRE_RECORD => collect_strings(section, field.nested, &item, out)?,
            _ => {}
        }
    }
    Ok(())
}

/// `value` as a record of `kind` of `section`.
pub(super) fn encode(
    section: u16,
    kind: u16,
    value: &Value,
    strings: &Strings,
) -> Result<Record, String> {
    let def = declared(section, kind)?;
    let mut record = Record::of(section, kind);
    for (field, item) in present(def, value)? {
        let mut bytes = Vec::new();
        match field.wire {
            WIRE_LOCAL_INDEX => uvarint(strings.index(text(def, &field, &item)?), &mut bytes),
            WIRE_UVARINT => uvarint(
                item.as_u64().ok_or(format!(
                    "{}.{}: not an unsigned integer",
                    def.name, field.name
                ))?,
                &mut bytes,
            ),
            WIRE_BOOL => bytes.push(u8::from(
                item.as_bool()
                    .ok_or(format!("{}.{}: not a boolean", def.name, field.name))?,
            )),
            WIRE_PACKED => {
                for element in items(def, &field, &item)? {
                    uvarint(strings.index(text(def, &field, element)?), &mut bytes);
                }
            }
            WIRE_RECORD if field.repeated => {
                for element in items(def, &field, &item)? {
                    encode(section, field.nested, element, strings)?.encode(&mut bytes);
                }
            }
            WIRE_RECORD => encode(section, field.nested, &item, strings)?.encode(&mut bytes),
            other => return Err(format!("{}.{}: wire type {other}", def.name, field.name)),
        }
        record = record.put(field.name, field.wire, &bytes);
    }
    Ok(record)
}

fn string_at(strings: &[String], index: u64, section: u16) -> Result<Value, AtlasError> {
    match usize::try_from(index).ok().and_then(|i| strings.get(i)) {
        Some(value) => Ok(Value::String(value.clone())),
        None => reject(format!(
            "section {section}: string index {index} outside the string table"
        )),
    }
}

/// Minimal uvarints packed back to back.
fn packed_uvarints(bytes: &[u8]) -> Result<Vec<u64>, AtlasError> {
    let mut out = Vec::new();
    let mut start = 0;
    for (at, byte) in bytes.iter().enumerate() {
        if byte & 0x80 == 0 {
            out.push(read_uvarint(&bytes[start..=at])?);
            start = at + 1;
        }
    }
    if start != bytes.len() {
        return reject("truncated packed uvarint");
    }
    Ok(out)
}

/// The embedded records of a `RECORD` field, each of the declared kind.
fn embedded<'a>(
    bytes: &'a [u8],
    section: u16,
    field: &FieldDef,
) -> Result<Vec<ParsedRecord<'a>>, AtlasError> {
    let records = parse_records(bytes, section)?;
    if records.is_empty() || (!field.repeated && records.len() != 1) {
        return reject(format!(
            "section {section}: field `{}` holds {} embedded records",
            field.name,
            records.len()
        ));
    }
    if let Some(r) = records.iter().find(|r| r.kind != field.nested) {
        return reject(format!(
            "section {section}: field `{}` embeds kind {}, declared {}",
            field.name, r.kind, field.nested
        ));
    }
    Ok(records)
}

/// The serde form of a record of `section`, rebuilt through its declared kind.
pub(super) fn decode(
    record: &ParsedRecord,
    section: u16,
    strings: &[String],
) -> Result<Value, AtlasError> {
    let Some(def) = schema::record(section, record.kind) else {
        return reject(format!(
            "section {section}: unknown record kind {}",
            record.kind
        ));
    };
    let fields = super::Fields::new(record, section)?;
    let mut map = Map::new();
    for field in def.fields {
        let Some(bytes) = fields.raw(field.name, field.wire)? else {
            if field.repeated {
                map.insert(field.name.to_owned(), Value::Array(Vec::new()));
            }
            continue;
        };
        let value = match field.wire {
            WIRE_LOCAL_INDEX => string_at(strings, read_uvarint(bytes)?, section)?,
            WIRE_UVARINT => Value::from(read_uvarint(bytes)?),
            WIRE_BOOL => match bytes {
                [0] => Value::Bool(false),
                [1] => Value::Bool(true),
                _ => {
                    return reject(format!(
                        "section {section}: `{}` is not a boolean",
                        field.name
                    ));
                }
            },
            WIRE_PACKED => Value::Array(
                packed_uvarints(bytes)?
                    .into_iter()
                    .map(|i| string_at(strings, i, section))
                    .collect::<Result<_, _>>()?,
            ),
            WIRE_RECORD => {
                let mut values = embedded(bytes, section, field)?
                    .iter()
                    .map(|r| decode(r, section, strings))
                    .collect::<Result<Vec<_>, _>>()?;
                if field.repeated {
                    Value::Array(values)
                } else {
                    values.pop().expect("exactly one")
                }
            }
            other => return reject(format!("section {section}: wire type {other}")),
        };
        map.insert(field.name.to_owned(), value);
    }
    if !is_enum(def) {
        return Ok(Value::Object(map));
    }
    let Some(Value::String(variant)) = map.remove("variant") else {
        return reject(format!("section {section}: {} without a variant", def.name));
    };
    match map.len() {
        0 => Ok(Value::String(variant)),
        1 if map.contains_key(&variant) => {
            let payload = map.remove(&variant).expect("present");
            let mut tagged = Map::new();
            tagged.insert(variant, payload);
            Ok(Value::Object(tagged))
        }
        _ => reject(format!(
            "section {section}: {} carries data for another variant than `{variant}`",
            def.name
        )),
    }
}
