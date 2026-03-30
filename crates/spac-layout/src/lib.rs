use spac_core::{
    validate_protocol_semantics, Diagnostic, FieldLayout, MetadataModel, ProtocolSpec,
    METADATA_SCHEMA_VERSION,
};
use std::collections::BTreeMap;

pub fn analyze_layout(
    protocol: &ProtocolSpec,
    bus_width_bits: u32,
) -> Result<MetadataModel, Vec<Diagnostic>> {
    let mut diagnostics = validate_protocol_semantics(protocol);

    if bus_width_bits == 0 || !bus_width_bits.is_power_of_two() {
        diagnostics.push(Diagnostic::error(
            "SPAC_BUS_WIDTH_INVALID",
            "$.bus_width_bits",
            "bus width must be a positive power of two",
        ));
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let mut bit_offset = 0_u64;
    let mut fields = Vec::with_capacity(protocol.fields.len());
    let mut semantic_bindings = BTreeMap::new();
    let bus_width = u64::from(bus_width_bits);

    for field in &protocol.fields {
        let bit_width = u64::from(field.bit_width);
        let first_flit = bit_offset / bus_width;
        let last_flit = (bit_offset + bit_width - 1) / bus_width;

        if let Some(semantic) = &field.semantic {
            semantic_bindings.insert(semantic.clone(), field.name.clone());
        }

        fields.push(FieldLayout {
            name: field.name.clone(),
            semantic: field.semantic.clone(),
            bit_offset,
            bit_width: field.bit_width,
            byte_offset: bit_offset / 8,
            flit_index: first_flit,
            crosses_flit_boundary: first_flit != last_flit,
        });

        bit_offset += bit_width;
    }

    Ok(MetadataModel {
        schema_version: METADATA_SCHEMA_VERSION.to_string(),
        protocol_name: protocol.name.clone(),
        bus_width_bits,
        total_header_bits: bit_offset,
        total_header_bytes: bit_offset.div_ceil(8),
        fields,
        semantic_bindings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use spac_core::{FieldSpec, PayloadKind, PayloadSpec};

    #[test]
    fn computes_offsets_and_flit_crossing() {
        let protocol = ProtocolSpec {
            name: "straddle".to_string(),
            fields: vec![
                FieldSpec {
                    name: "dst".to_string(),
                    bit_width: 7,
                    semantic: Some("routing_key".to_string()),
                },
                FieldSpec {
                    name: "wide".to_string(),
                    bit_width: 4,
                    semantic: None,
                },
            ],
            payload: Some(PayloadSpec {
                kind: PayloadKind::Bytes,
            }),
        };

        let metadata = analyze_layout(&protocol, 8).expect("layout");

        assert_eq!(metadata.total_header_bits, 11);
        assert_eq!(metadata.total_header_bytes, 2);
        assert_eq!(metadata.fields[0].bit_offset, 0);
        assert_eq!(metadata.fields[1].bit_offset, 7);
        assert!(metadata.fields[1].crosses_flit_boundary);
        assert_eq!(
            metadata
                .semantic_bindings
                .get("routing_key")
                .map(String::as_str),
            Some("dst")
        );
    }

    #[test]
    fn rejects_non_power_of_two_bus_width() {
        let protocol = ProtocolSpec {
            name: "basic".to_string(),
            fields: vec![FieldSpec {
                name: "dst".to_string(),
                bit_width: 8,
                semantic: Some("routing_key".to_string()),
            }],
            payload: None,
        };

        let diagnostics = analyze_layout(&protocol, 24).expect_err("invalid bus width");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_BUS_WIDTH_INVALID"));
    }
}
