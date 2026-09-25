//! Every `.atlas` schema definition this reader has ever written, oldest first (G86, ADR 0035).
//!
//! A generation is a line `generation <id>` followed by the `definition_text` of every section.
//! Append a new generation before changing `schema::SECTIONS`, and never edit a recorded one:
//! `the_current_schema_is_the_latest_recorded_generation` and
//! `every_recorded_schema_conforms_to_the_current_one` enforce both.

pub const HISTORY: &str = r#"generation G68
atlas.wire.v1/root-manifest
record 1 manifest
field 1 wire_version 1 required
field 2 genome_schema 6 required
field 3 genome_hash 7 required
field 4 census_digest 7 required
field 5 revision 6 required
field 6 certificate_id 6 required
field 7 seal 6 required
field 8 tool 6 required
field 9 mode 6 required
record 2 section-commitment
field 1 section_type 1 required
field 2 schema_id 1 required
field 3 content_hash 7 required
field 4 record_count 1 required
atlas.wire.v1/string-table
record 1 string
field 1 value 6 required
atlas.wire.v1/census-facts
record 1 fact
field 1 id 9 required
field 2 kind 9 required
field 3 status 9 required
field 4 subject 9 required
field 5 predicate 9 required
field 6 object 9 required
field 7 source_path 9 required
field 8 revision 9 optional
field 9 extractor 9 required
field 10 span 9 optional
depends string-table 3691c0b2326dcf167420571c120aaf94c19702e42455dce2c86f4f901328426f
atlas.wire.v1/declared-nodes
record 1 node
field 1 name 9 required
field 2 kind 9 required
field 3 origin 9 required
depends string-table 3691c0b2326dcf167420571c120aaf94c19702e42455dce2c86f4f901328426f
atlas.wire.v1/declared-edges
record 1 edge
field 1 from 9 required
field 2 relation 9 required
field 3 to 9 required
depends string-table 3691c0b2326dcf167420571c120aaf94c19702e42455dce2c86f4f901328426f
atlas.wire.v1/semantic-obligations
record 1 obligation
field 1 artifact 9 required
field 2 dimension 9 required
field 3 extractor 9 required
field 4 extractor_version 9 required
field 5 status 9 required
depends string-table 3691c0b2326dcf167420571c120aaf94c19702e42455dce2c86f4f901328426f
atlas.wire.v1/census-certificate
record 1 certificate
field 1 certificate_id 9 required
field 2 state 9 required
record 2 blocker
field 1 text 9 required
depends string-table 3691c0b2326dcf167420571c120aaf94c19702e42455dce2c86f4f901328426f
"#;
