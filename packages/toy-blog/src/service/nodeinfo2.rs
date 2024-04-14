use std::intrinsics::transmute;
use serde::{Serialize, Serializer};

#[derive(Serialize)]
pub struct Report<'a> {
    pub links: &'a [ReportEntry<'a>]
}

#[derive(Serialize)]
pub struct ReportEntry<'a> {
    /// must be URL, points to schema
    rel: &'a str,
    /// must be actual provider
    href: &'a str,
}

impl<'a> ReportEntry<'a> {
    pub fn new(version: NodeInfo2Version, href: &'a str) -> Self {
        Self {
            rel: version.schema_url(),
            href,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInfo2<'a> {
    /// is open to registration?
    pub(crate) open_registrations: bool,
    /// supported protocols (i.e. ActivityPub)
    pub(crate) protocols: ProtocolTable<'a>,
    pub(crate) software: SoftwareInformation<'a>,
    pub(crate) usage: UsageSnapshot,
    pub(crate) services: ConnectingExternalService<'a>,
    pub(crate) metadata: serde_json::Value,
    pub(crate) version: SerializeAsRefStr<NodeInfo2Version>,
    pub(crate) instance: InstanceInformation<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectingExternalService<'a> {
    pub(crate) inbound: &'a [ConnectingExternalIncomingServiceEntry],
    pub(crate) outbound: &'a [ConnectingExternalOutgoingServiceEntry],
}

#[derive(Serialize)]
#[non_exhaustive] // it may be added in later version.
pub enum ConnectingExternalIncomingServiceEntry {}

#[derive(Serialize)]
#[non_exhaustive] // it may be added in later version.
pub enum ConnectingExternalOutgoingServiceEntry {}
#[derive(Eq, PartialEq, Hash, Debug, Ord, PartialOrd, Copy, Clone)]
pub enum NodeInfo2Version {
    V2_0,
    V2_1,
    V2_2,
}

impl NodeInfo2Version {
    pub(crate) fn schema_url(self) -> &'static str {
        match self {
            NodeInfo2Version::V2_0 => "https://nodeinfo.diaspora.software/ns/schema/2.0",
            NodeInfo2Version::V2_1 => "https://nodeinfo.diaspora.software/ns/schema/2.1",
            NodeInfo2Version::V2_2 => "https://nodeinfo.diaspora.software/ns/schema/2.2",
        }
    }
}

impl AsRef<str> for NodeInfo2Version {
    fn as_ref(&self) -> &str {
        match self {
            NodeInfo2Version::V2_0 => "2.0",
            NodeInfo2Version::V2_1 => "2.1",
            NodeInfo2Version::V2_2 => "2.2",
        }
    }
}

/// Represents supported protocol in the array.
/// Note: `#[repr(transparent)]` spec. is private to this `well_known` module. Other modules including
/// external crate must not depend on this. Instead, use appropriate function to convert.
#[derive(Serialize)]
#[repr(transparent)]
pub struct CustomizedProtocolTable<'a>([&'a str]);

#[derive(Serialize)]
#[repr(transparent)]
pub struct ValidProtocolTable([SerializeAsRefStr<ValidProtocol>]);

impl<'a> From<&'a [ValidProtocol]> for &'a ValidProtocolTable {
    fn from(value: &'a [ValidProtocol]) -> Self {
        // SAFETY: layout(ValidProtocolTable) == layout([SerializeAsRefStr<ValidProtocol>])
        //         and layout([SerializeAsRefStr<ValidProtocol>]) == layout([ValidProtocol])
        //         because forall T. layout(SerializeAsRefStr<T>) == layout(T) and
        //         forall T, U. layout(T) == layout(U) -> layout([T]) == layout([U])
        unsafe { transmute(value) }
    }
}

impl<'a, const N: usize> From<&'a [ValidProtocol; N]> for &'a ValidProtocolTable {
    fn from(value: &'a [ValidProtocol; N]) -> Self {
        Self::from(value as &'a [ValidProtocol])
    }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ProtocolTable<'a> {
    Custom(&'a CustomizedProtocolTable<'a>),
    Valid(&'a ValidProtocolTable),
}

impl ProtocolTable<'_> {
    fn is_empty(&self) -> bool {
        match self {
            ProtocolTable::Custom(t) => t.0.is_empty(),
            ProtocolTable::Valid(t) => t.0.is_empty(),
        }
    }
}

#[repr(transparent)] // private to this module
pub struct SerializeAsRefStr<T>(pub T);

impl<T: AsRef<str>> Serialize for SerializeAsRefStr<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        serializer.serialize_str(self.0.as_ref())
    }
}

#[derive(Eq, PartialEq, Debug, Hash)]
#[non_exhaustive] // it may be added in later version.
pub enum ValidProtocol {
    ActivityPub,
    BuddyCloud,
    DFRN,
    Dlaspora,
    Libertree,
    Nostr,
    OStatus,
    PumpIo,
    Tent,
    Xmpp,
    Zot,
}

impl AsRef<str> for ValidProtocol {
    fn as_ref(&self) -> &str {
        match self {
            ValidProtocol::ActivityPub => "activitypub",
            ValidProtocol::BuddyCloud => "buddycloud",
            ValidProtocol::DFRN => "dfrn",
            ValidProtocol::Dlaspora => "dlaspora",
            ValidProtocol::Libertree => "libertree",
            ValidProtocol::Nostr => "nostr",
            ValidProtocol::OStatus => "ostatus",
            ValidProtocol::PumpIo => "pumpio",
            ValidProtocol::Tent => "tent",
            ValidProtocol::Xmpp => "xmpp",
            ValidProtocol::Zot => "zot",
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareInformation<'a> {
    pub(crate) name: &'a str,
    pub(crate) version: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repository: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) homepage: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub(crate) users: ActiveUserReporter,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) local_posts: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) local_comments: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[derive(Eq, PartialEq)]
pub struct ActiveUserReporter {
    pub(crate) total: usize,
    #[serde(rename = "activeHalfyear")]
    pub(crate) active_half_year: usize,
    pub(crate) active_month: usize,
    pub(crate) active_week: usize,
}

#[derive(Serialize)]
#[derive(Eq, PartialEq)]
pub struct InstanceInformation<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<&'a str>,
}
