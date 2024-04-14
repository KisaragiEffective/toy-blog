use actix_web::{get, Responder};
use actix_web::web::Data;

use crate::service::hosting::{DoRewriteHttps, HostingUrlBaseWithoutSchema};

#[get("/nodeinfo")]
#[cfg(feature = "unstable_nodeinfo2")]
pub async fn node_info(base_uri: Data<HostingUrlBaseWithoutSchema>, use_https: Data<DoRewriteHttps>) -> impl Responder {
    use crate::service::nodeinfo2::Report;
    use crate::service::nodeinfo2::{NodeInfo2Version, ReportEntry};

    let mut actual_url = "".to_string();
    let use_https = use_https.into_inner();
    let base_uri = base_uri.into_inner();
    if use_https.rewrite {
        actual_url += "https://";
    } else {
        actual_url += "http://";
    }

    actual_url += base_uri.host();
    actual_url += "/";
    actual_url += base_uri.path();
    actual_url += "/nodeinfo/2.2";

    let x = (Report {
        links: &[
            ReportEntry::new(NodeInfo2Version::V2_2, &actual_url)
        ],
    });

    serde_json::to_string(&x)
}