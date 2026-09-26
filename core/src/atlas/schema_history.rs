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
generation G147
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
record 2 FunctionIdentity
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=111
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 3 FunctionSignature
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=113
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 4 Symbol
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=109
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 5 Type
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=108
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 6 Call
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=114
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 7 ControlFlow
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=116
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 8 DataFlow
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=117
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 9 State
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=118
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 10 Effect
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=119
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 11 Ownership
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=120
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 12 Concurrency
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=121
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 13 Persistence
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=122
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 100 revision
field 1 kind 9 required
field 2 value 9 required
record 101 scope
field 1 segments 11 optional repeated element=9
record 102 extractor
field 1 id 9 required
field 2 version 9 required
record 103 provenance
field 1 source_path 9 required
field 2 source_revision 10 optional record=100
field 3 extractor 9 required
field 4 content_hash 9 optional
field 5 span 9 optional
record 104 source-span
field 1 path 9 required
field 2 line 1 required
field 3 column 1 required
record 105 place-ref
field 1 variant 9 required
field 2 Resolved 10 optional record=106
record 106 place-resolved
field 1 dimension 9 required
field 2 record_id 9 required
record 107 documentation
field 1 summary 9 required
field 2 lines 1 required
record 108 type-identity
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 scope 10 required record=101
field 4 name 9 required
field 5 canonical 9 optional
field 6 path 9 required
record 109 symbol-identity
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 scope 10 required record=101
field 4 name 9 required
field 5 role 9 required
field 6 path 9 required
field 7 documentation 10 optional record=107
record 110 function-owner
field 1 target 10 optional record=108
field 2 trait_path 9 optional
record 111 function-identity
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 language 9 required
field 4 scope 10 required record=101
field 5 symbol 10 required record=109
field 6 span 10 required record=104
field 7 generated 12 required
field 8 declaration_kind 9 required
field 9 owner 10 required record=110
field 10 generics 11 optional repeated element=9
record 112 function-parameter
field 1 name 9 required
field 2 type_identity 10 required record=108
record 113 function-signature
field 1 function 10 required record=111
field 2 parameters 10 optional record=112 repeated
field 3 return_type 10 optional record=108
field 4 generics 11 optional repeated element=9
field 5 abi 9 optional
field 6 visibility 9 required
field 7 is_async 12 required
field 8 is_unsafe 12 required
field 9 is_extern 12 required
field 10 body_fingerprint 9 optional
record 114 call-site
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 span 10 required record=104
field 5 dispatch 9 required
field 6 callees 11 optional repeated element=9
field 7 arguments 10 optional record=105 repeated
field 8 result 10 required record=105
field 9 callee_spelling 9 optional
record 115 control-flow-edge
field 1 kind 9 required
field 2 target 9 optional
record 116 control-flow-block
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 block_index 1 required
field 5 kind 9 required
field 6 is_entry 12 required
field 7 successors 10 optional record=115 repeated
record 117 value
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 name 9 required
field 5 span 10 required record=104
field 6 role 9 required
field 7 is_parameter 12 required
field 8 is_return_flow 12 required
field 9 resolution 9 required
field 10 resolved_definition 9 optional
field 11 projection 11 optional repeated element=9
record 118 state-access
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 scope 10 required record=101
field 5 name 9 required
field 6 span 10 required record=104
field 7 kind 9 required
field 8 resolution 9 required
record 119 effect
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 category 9 required
field 5 span 10 required record=104
record 120 ownership
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 name 9 required
field 5 span 10 required record=104
field 6 kind 9 required
field 7 resolution 9 required
record 121 concurrency
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 kind 9 required
field 5 span 10 required record=104
record 122 persistence
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 kind 9 required
field 5 span 10 required record=104
field 6 place 10 required record=105
field 7 resolution 9 required
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
atlas.wire.v1/evidence
record 1 evidence
field 1 id 9 required
field 2 kind 9 required
field 3 path 9 required
field 4 summary 9 required
field 5 revision 10 optional record=2
record 2 revision
field 1 kind 9 required
field 2 value 9 required
depends string-table 3691c0b2326dcf167420571c120aaf94c19702e42455dce2c86f4f901328426f
atlas.wire.v1/diagnostics
record 1 diagnostic
field 1 id 9 required
field 2 code 9 required
field 3 dimension 9 optional
field 4 message 9 required
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
generation G153
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
record 2 FunctionIdentity
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=111
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 3 FunctionSignature
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=113
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 4 Symbol
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=109
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 5 Type
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=108
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 6 Call
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=114
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 7 ControlFlow
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=116
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 8 DataFlow
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=117
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 9 State
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=118
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 10 Effect
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=119
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 11 Ownership
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=120
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 12 Concurrency
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=121
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 13 Persistence
field 1 record_id 9 required
field 2 dimension 9 required
field 3 status 9 required
field 4 subject 10 required record=122
field 5 scope 10 required record=101
field 6 repository 9 required
field 7 revision 10 required record=100
field 8 extractor 10 required record=102
field 9 evidence_refs 11 optional repeated element=9
field 10 provenance 10 required record=103
record 100 revision
field 1 kind 9 required
field 2 value 9 required
record 101 scope
field 1 segments 11 optional repeated element=9
record 102 extractor
field 1 id 9 required
field 2 version 9 required
record 103 provenance
field 1 source_path 9 required
field 2 source_revision 10 optional record=100
field 3 extractor 9 required
field 4 content_hash 9 optional
field 5 span 9 optional
record 104 source-span
field 1 path 9 required
field 2 line 1 required
field 3 column 1 required
record 105 place-ref
field 1 variant 9 required
field 2 Resolved 10 optional record=106
record 106 place-resolved
field 1 dimension 9 required
field 2 record_id 9 required
record 107 documentation
field 1 summary 9 required
field 2 lines 1 required
record 123 declaration
field 1 item 9 required
field 2 visibility 9 required
field 3 derives 11 optional repeated element=9
field 4 attributes 11 optional repeated element=9
field 5 shape 9 optional
record 108 type-identity
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 scope 10 required record=101
field 4 name 9 required
field 5 canonical 9 optional
field 6 path 9 required
record 109 symbol-identity
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 scope 10 required record=101
field 4 name 9 required
field 5 role 9 required
field 6 path 9 required
field 7 documentation 10 optional record=107
field 8 declaration 10 optional record=123
record 110 function-owner
field 1 target 10 optional record=108
field 2 trait_path 9 optional
record 111 function-identity
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 language 9 required
field 4 scope 10 required record=101
field 5 symbol 10 required record=109
field 6 span 10 required record=104
field 7 generated 12 required
field 8 declaration_kind 9 required
field 9 owner 10 required record=110
field 10 generics 11 optional repeated element=9
record 112 function-parameter
field 1 name 9 required
field 2 type_identity 10 required record=108
record 113 function-signature
field 1 function 10 required record=111
field 2 parameters 10 optional record=112 repeated
field 3 return_type 10 optional record=108
field 4 generics 11 optional repeated element=9
field 5 abi 9 optional
field 6 visibility 9 required
field 7 is_async 12 required
field 8 is_unsafe 12 required
field 9 is_extern 12 required
field 10 body_fingerprint 9 optional
record 114 call-site
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 span 10 required record=104
field 5 dispatch 9 required
field 6 callees 11 optional repeated element=9
field 7 arguments 10 optional record=105 repeated
field 8 result 10 required record=105
field 9 callee_spelling 9 optional
record 115 control-flow-edge
field 1 kind 9 required
field 2 target 9 optional
record 116 control-flow-block
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 block_index 1 required
field 5 kind 9 required
field 6 is_entry 12 required
field 7 successors 10 optional record=115 repeated
record 117 value
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 name 9 required
field 5 span 10 required record=104
field 6 role 9 required
field 7 is_parameter 12 required
field 8 is_return_flow 12 required
field 9 resolution 9 required
field 10 resolved_definition 9 optional
field 11 projection 11 optional repeated element=9
record 118 state-access
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 scope 10 required record=101
field 5 name 9 required
field 6 span 10 required record=104
field 7 kind 9 required
field 8 resolution 9 required
record 119 effect
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 category 9 required
field 5 span 10 required record=104
record 120 ownership
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 name 9 required
field 5 span 10 required record=104
field 6 kind 9 required
field 7 resolution 9 required
record 121 concurrency
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 kind 9 required
field 5 span 10 required record=104
record 122 persistence
field 1 repository 9 required
field 2 revision 10 required record=100
field 3 function 9 required
field 4 kind 9 required
field 5 span 10 required record=104
field 6 place 10 required record=105
field 7 resolution 9 required
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
atlas.wire.v1/evidence
record 1 evidence
field 1 id 9 required
field 2 kind 9 required
field 3 path 9 required
field 4 summary 9 required
field 5 revision 10 optional record=2
record 2 revision
field 1 kind 9 required
field 2 value 9 required
depends string-table 3691c0b2326dcf167420571c120aaf94c19702e42455dce2c86f4f901328426f
atlas.wire.v1/diagnostics
record 1 diagnostic
field 1 id 9 required
field 2 code 9 required
field 3 dimension 9 optional
field 4 message 9 required
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
