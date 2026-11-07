// One vocabulary drives decoding, serialization, effective ownership and counting.
// Legacy fields retain their old admission only at previously typed boundaries.
macro_rules! schema_fields {
    ($apply:ident) => {
        $apply! {
            legacy {
                _type => "type": scalar,
                _ref => "$ref": scalar,
                format => "format": scalar,
                nullable => "nullable": scalar,
                description => "description": scalar,
                content_media_type => "contentMediaType": scalar,
                content_encoding => "contentEncoding": scalar,
                content_schema => "contentSchema": legacy_child
            }
            structural {
                properties => "properties": schema_map,
                pattern_properties => "patternProperties": schema_map,
                required => "required": strings,
                min_properties => "minProperties": scalar,
                max_properties => "maxProperties": scalar,
                additional_properties => "additionalProperties": child,
                unevaluated_properties => "unevaluatedProperties": child,
                property_names => "propertyNames": child,
                all_of => "allOf": schema_array,
                any_of => "anyOf": schema_array,
                one_of => "oneOf": schema_array,
                not => "not": child,
                if_schema => "if": child,
                then_schema => "then": child,
                else_schema => "else": child,
                dependent_required => "dependentRequired": string_map,
                dependent_schemas => "dependentSchemas": schema_map,
                dependencies => "dependencies": dependencies,
                defs => "$defs": schema_map,
                definitions => "definitions": schema_map,
                items => "items": items,
                prefix_items => "prefixItems": schema_array,
                additional_items => "additionalItems": child,
                unevaluated_items => "unevaluatedItems": child,
                contains => "contains": child,
                #[cfg(feature = "lossless-schema-keywords")]
                schema_format => "schemaFormat": scalar,
                #[cfg(feature = "lossless-schema-keywords")]
                schema => "schema": child,
                #[cfg(feature = "lossless-schema-keywords")]
                external_docs => "externalDocs": external_docs,
                #[cfg(feature = "lossless-schema-keywords")]
                discriminator => "discriminator": component
            }
        }
    };
}

pub(crate) use schema_fields;
