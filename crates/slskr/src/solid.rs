//! Solid profile parsing shared by the HTTP compatibility route.

use oxixml_io::{RdfFormat, RdfParser};
use oxixml_model::{NamedOrBlankNode, Term};

const SOLID_OIDC_ISSUER: &str = "http://www.w3.org/ns/solid/terms#oidcIssuer";
const MAX_PROFILE_QUADS: usize = 16_384;

/// Extract OIDC issuer IRIs from a bounded Solid WebID profile.
///
/// The frozen slskdN resolver accepts Turtle, JSON-LD, and RDF/XML and resolves
/// relative profile IRIs against the requested WebID document. Unknown media
/// types intentionally use the same Turtle fallback as the oracle.
pub(crate) fn extract_oidc_issuers(
    body: &[u8],
    content_type: Option<&str>,
    web_id: &str,
) -> Result<Vec<String>, String> {
    let format = content_type
        .and_then(RdfFormat::from_media_type)
        .unwrap_or(RdfFormat::Turtle);
    let parser = RdfParser::from_format(format)
        .with_base_iri(web_id)
        .map_err(|error| format!("invalid WebID base IRI: {error}"))?
        .without_named_graphs()
        .for_slice(body);

    let mut issuers = Vec::new();
    for (index, quad) in parser.enumerate() {
        if index >= MAX_PROFILE_QUADS {
            return Err(format!(
                "Solid profile contains more than {MAX_PROFILE_QUADS} RDF statements"
            ));
        }
        let quad = quad.map_err(|error| format!("invalid Solid RDF profile: {error}"))?;
        if !matches!(
            &quad.subject,
            NamedOrBlankNode::NamedNode(subject) if subject.as_str() == web_id
        ) || quad.predicate.as_str() != SOLID_OIDC_ISSUER
        {
            continue;
        }
        if let Term::NamedNode(issuer) = quad.object {
            issuers.push(issuer.into_string().into());
        }
    }
    if matches!(format, RdfFormat::JsonLd { .. }) {
        preserve_json_ld_issuer_multiplicity(body, web_id, &mut issuers);
    }
    Ok(issuers)
}

/// JSON-LD's RDF graph model is set-like, while the frozen slskdN resolver
/// preserves repeated values in an explicit issuer array. Restore that narrow
/// compatibility detail after the standards-compliant parser has validated and
/// expanded the document.
fn preserve_json_ld_issuer_multiplicity(body: &[u8], web_id: &str, issuers: &mut Vec<String>) {
    let Ok(document) = serde_json::from_slice::<serde_json::Value>(body) else {
        return;
    };
    let mut raw_issuers = Vec::new();
    collect_json_ld_issuer_values(&document, web_id, &mut raw_issuers);
    if raw_issuers.len() <= issuers.len()
        || raw_issuers
            .iter()
            .any(|issuer| !issuers.iter().any(|value| value == issuer))
    {
        return;
    }
    let parsed = issuers.clone();
    issuers.clear();
    for issuer in raw_issuers {
        issuers.push(issuer);
    }
    for issuer in parsed {
        if !issuers.iter().any(|value| value == &issuer) {
            issuers.push(issuer);
        }
    }
}

fn collect_json_ld_issuer_values(
    value: &serde_json::Value,
    web_id: &str,
    issuers: &mut Vec<String>,
) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                collect_json_ld_issuer_values(value, web_id, issuers);
            }
        }
        serde_json::Value::Object(object) => {
            if object.get("@id").and_then(serde_json::Value::as_str) == Some(web_id) {
                for key in ["solid:oidcIssuer", SOLID_OIDC_ISSUER] {
                    let Some(value) = object.get(key) else {
                        continue;
                    };
                    let values = match value {
                        serde_json::Value::Array(values) => values.as_slice(),
                        value => std::slice::from_ref(value),
                    };
                    for value in values {
                        if let Some(issuer) = value
                            .as_object()
                            .and_then(|object| object.get("@id"))
                            .and_then(serde_json::Value::as_str)
                        {
                            issuers.push(issuer.to_owned());
                        }
                    }
                }
            }
            for key in ["@graph", "@included"] {
                if let Some(value) = object.get(key) {
                    collect_json_ld_issuer_values(value, web_id, issuers);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::extract_oidc_issuers;

    const WEB_ID: &str = "https://profile.example/profile/card#me";

    #[test]
    fn turtle_profile_extracts_relative_and_absolute_issuers() {
        let profile = br#"@prefix solid: <http://www.w3.org/ns/solid/terms#>.

<#me>
  solid:oidcIssuer <https://issuer.example/oidc>;
  solid:oidcIssuer <../issuer-two>.
"#;

        let issuers = extract_oidc_issuers(profile, Some("text/turtle; charset=utf-8"), WEB_ID)
            .expect("valid Turtle profile");
        assert_eq!(
            issuers,
            [
                "https://issuer.example/oidc".to_owned(),
                "https://profile.example/issuer-two".to_owned()
            ]
        );
    }

    #[test]
    fn json_ld_profile_preserves_duplicate_array_issuers() {
        let profile = br#"{
          "@context": {"solid": "http://www.w3.org/ns/solid/terms#"},
          "@id": "https://profile.example/profile/card#me",
          "solid:oidcIssuer": [
            {"@id": "https://issuer.example/oidc"},
            {"@id": "https://issuer.example/oidc"}
          ]
        }"#;

        let issuers = extract_oidc_issuers(profile, Some("application/ld+json"), WEB_ID)
            .expect("valid JSON-LD profile");
        assert_eq!(
            issuers,
            ["https://issuer.example/oidc", "https://issuer.example/oidc"]
        );
    }

    #[test]
    fn rdf_xml_profile_extracts_issuer() {
        let profile = br#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:solid="http://www.w3.org/ns/solid/terms#">
  <rdf:Description rdf:about="https://profile.example/profile/card#me">
    <solid:oidcIssuer rdf:resource="https://issuer.example/oidc" />
  </rdf:Description>
</rdf:RDF>"#;

        let issuers = extract_oidc_issuers(profile, Some("application/rdf+xml"), WEB_ID)
            .expect("valid RDF/XML profile");
        assert_eq!(issuers, ["https://issuer.example/oidc"]);
    }

    #[test]
    fn malformed_profile_is_rejected_instead_of_reported_as_empty() {
        let error = extract_oidc_issuers(
            br#"@prefix solid: <http://www.w3.org/ns/solid/terms#>.
<#me> solid:oidcIssuer ."#,
            Some("text/turtle"),
            WEB_ID,
        )
        .expect_err("malformed RDF must fail closed");
        assert!(error.contains("invalid Solid RDF profile"), "{error}");
    }
}
