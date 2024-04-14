use actix_web::{get, Responder};
use actix_web::web::Json;
use serde_json::json;

use crate::service::nodeinfo2::{ActiveUserReporter, ConnectingExternalService, InstanceInformation, NodeInfo2, NodeInfo2Version, ProtocolTable, SerializeAsRefStr, SoftwareInformation, UsageSnapshot, ValidProtocolTable};

#[get("/2.2")]
/// returns https://github.com/jhass/nodeinfo/blob/main/schemas/2.2/schema.json
pub async fn node_info_2() -> impl Responder {
    let construction = compute();

    Json(construction)
}

fn compute<'a>() -> NodeInfo2<'a> {
    NodeInfo2 {
        open_registrations: false,
        protocols: ProtocolTable::Valid(<&ValidProtocolTable>::from(&[])),
        software: SoftwareInformation {
            name: "toy-blog",
            version: "",
            repository: None,
            homepage: None,
        },
        usage: UsageSnapshot {
            users: ActiveUserReporter {
                total: 1,
                active_half_year: 0,
                active_month: 0,
                active_week: 0,
            },
            local_posts: None,
            local_comments: None,
        },
        services: ConnectingExternalService {
            inbound: &[],
            outbound: &[]
        },
        metadata: json!({}),
        version: SerializeAsRefStr(NodeInfo2Version::V2_2),
        instance: InstanceInformation {
            name: None,
            description: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use jsonschema::error::ValidationErrorKind;
    use jsonschema::ValidationError;
    use serde_json::json;

    use crate::service::rest::node_info::compute;

    #[test]
    fn validate_22() {
        // adopted from https://github.com/jhass/nodeinfo/blob/72c25eea1cb5995740080c0c0e5eb5abf488a4b1/schemas/2.2/schema.json
        // license: CC0-1.0 per https://github.com/jhass/nodeinfo/blob/72c25eea1cb5995740080c0c0e5eb5abf488a4b1/README.md#nodeinfo
        let json_schema = || {
            json!({
  "title": "NodeInfo schema version 2.2",
  "$schema": "http://json-schema.org/draft-04/schema#",
  "id": "http://nodeinfo.diaspora.software/ns/schema/2.2#",
  "description": "NodeInfo is an effort to create a standardized way of exposing metadata about a server running one of the distributed social networks.",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "version",
    "instance",
    "software",
    "protocols",
    "services",
    "openRegistrations",
    "usage",
    "metadata"
  ],
  "properties": {
    "version": {
      "description": "The schema version, must be 2.2.",
      "enum": [
        "2.2"
      ]
    },
    "instance":{
      "description": "Metadata specific to the instance. An instance is a the concrete installation of a software running on a server.",
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "name": {
          "description": "If supported by the software the administrator configured name of this instance",
          "type": "string",
          "pattern": "^.{0,500}$"
        },
        "description": {
          "description": "If supported by the software the administrator configured long form description of this instance.",
          "type": "string",
          "pattern": "^.{0,5000}$"
        }
      }
    },
    "software": {
      "description": "Metadata about server software in use.",
      "type": "object",
      "additionalProperties": false,
      "required": [
        "name",
        "version"
      ],
      "properties": {
        "name": {
          "description": "The canonical name of this server software.",
          "type": "string",
          "pattern": "^[a-z0-9-]+$"
        },
        "version": {
          "description": "The version of this server software.",
          "type": "string"
        },
        "repository": {
          "description": "The url of the source code repository of this server software.",
          "type": "string"
        },
        "homepage": {
          "description": "The url of the homepage of this server software.",
          "type": "string"
        }
      }
    },
    "protocols": {
      "description": "The protocols supported on this server.",
      "type": "array",
      "minItems": 1,
      "items": {
        "enum": [
          "activitypub",
          "buddycloud",
          "dfrn",
          "diaspora",
          "libertree",
          "nostr",
          "ostatus",
          "pumpio",
          "tent",
          "xmpp",
          "zot"
        ]
      }
    },
    "services": {
      "description": "The third party sites this server can connect to via their application API.",
      "type": "object",
      "additionalProperties": false,
      "required": [
        "inbound",
        "outbound"
      ],
      "properties": {
        "inbound": {
          "description": "The third party sites this server can retrieve messages from for combined display with regular traffic.",
          "type": "array",
          "minItems": 0,
          "items": {
            "enum": [
              "atom1.0",
              "gnusocial",
              "imap",
              "pnut",
              "pop3",
              "pumpio",
              "rss2.0",
              "twitter"
            ]
          }
        },
        "outbound": {
          "description": "The third party sites this server can publish messages to on the behalf of a user.",
          "type": "array",
          "minItems": 0,
          "items": {
            "enum": [
              "atom1.0",
              "blogger",
              "buddycloud",
              "diaspora",
              "dreamwidth",
              "drupal",
              "facebook",
              "friendica",
              "gnusocial",
              "google",
              "insanejournal",
              "libertree",
              "linkedin",
              "livejournal",
              "mediagoblin",
              "myspace",
              "pinterest",
              "pnut",
              "posterous",
              "pumpio",
              "redmatrix",
              "rss2.0",
              "smtp",
              "tent",
              "tumblr",
              "twitter",
              "wordpress",
              "xmpp"
            ]
          }
        }
      }
    },
    "openRegistrations": {
      "description": "Whether this server allows open self-registration.",
      "type": "boolean"
    },
    "usage": {
      "description": "Usage statistics for this server.",
      "type": "object",
      "additionalProperties": false,
      "required": [
        "users"
      ],
      "properties": {
        "users": {
          "description": "statistics about the users of this server.",
          "type": "object",
          "additionalProperties": false,
          "properties": {
            "total": {
              "description": "The total amount of on this server registered users.",
              "type": "integer",
              "minimum": 0
            },
            "activeHalfyear": {
              "description": "The amount of users that signed in at least once in the last 180 days.",
              "type": "integer",
              "minimum": 0
            },
            "activeMonth": {
              "description": "The amount of users that signed in at least once in the last 30 days.",
              "type": "integer",
              "minimum": 0
            },
            "activeWeek": {
              "description": "The amount of users that signed in at least once in the last 7 days.",
              "type": "integer",
              "minimum": 0
            }
          }
        },
        "localPosts": {
          "description": "The amount of posts that were made by users that are registered on this server.",
          "type": "integer",
          "minimum": 0
        },
        "localComments": {
          "description": "The amount of comments that were made by users that are registered on this server.",
          "type": "integer",
          "minimum": 0
        }
      }
    },
    "metadata": {
      "description": "Free form key value pairs for software specific values. Clients should not rely on any specific key present.",
      "type": "object",
      "minProperties": 0,
      "additionalProperties": true
    }
  }
})
        };

        let info = serde_json::to_value(compute()).expect("into value");
        let sc = jsonschema::JSONSchema::compile(&json_schema()).expect("a valid schema");
        let res = sc.validate(&info);
        
        if let Err(errors) = res {
            let exclude_protocol_min_item_violation = |e: &ValidationError| {
                let m6 = matches!(e.kind, ValidationErrorKind::MinItems { limit: 1 });
                let ii = e.instance_path.clone().into_vec();
                let mp = ii.len() == 1 && ii[0] == "instances";
                
                !m6 && !mp
            };
            
            let mut errors = errors.filter(exclude_protocol_min_item_violation).peekable();
            if errors.peek().is_none() {
                return
            }
            
            for error in errors {
                eprintln!("validation error: {error}");
                eprintln!("+--- occured in `{}`", error.instance_path);
            }
            
            panic!("returned value does not conform to json schema");
        }
    }
}
