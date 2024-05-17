pub struct HostingUrlBaseWithoutSchema {
    hostname: String,
    paths: String,
}

impl HostingUrlBaseWithoutSchema {
    /// paths must start with forward slash.
    fn new(hostname: String, path: String) -> Result<Self, ()> {
        // 絶対スラッシュで始まってほしい
        // かつスラッシュで終わってほしくないが、スラッシュだけの場合は許可
        if path.starts_with('/') && (!path.ends_with('/') || path == "/") {
            Ok(Self {
                hostname,
                paths: path
            })
        } else {
            Err(())
        }
    }
    
    pub fn host(&self) -> &str {
        &self.hostname
    }
    
    pub fn path(&self) -> &str {
        &self.paths
    }
}

pub struct DoRewriteHttps {
    pub rewrite: bool,
}

#[cfg(test)]
mod tests {
    use crate::service::hosting::HostingUrlBaseWithoutSchema;

    #[test]
    #[should_panic]
    fn empty_err() {
        HostingUrlBaseWithoutSchema::new("123".to_string(), "".to_string()).expect("non empty path");
    }

    #[test]
    fn single_slash_ok() {
        let a = HostingUrlBaseWithoutSchema::new("123".to_string(), "/".to_string()).expect("infallible");
        assert_eq!(a.host(), "123");
        assert_eq!(a.path(), "/");
    }

    #[test]
    fn single_level_ok() {
        let a = HostingUrlBaseWithoutSchema::new("123".to_string(), "/abc".to_string()).expect("infallible");
        assert_eq!(a.host(), "123");
        assert_eq!(a.path(), "/abc");
    }
    
    #[test]
    fn multi_level_ok() {
        let a = HostingUrlBaseWithoutSchema::new("123".to_string(), "/abc/def/ghi".to_string()).expect("infallible");
        assert_eq!(a.host(), "123");
        assert_eq!(a.path(), "/abc/def/ghi");
    }
    
    #[test]
    #[should_panic]
    fn trailing_slash_err() {
        HostingUrlBaseWithoutSchema::new("123".to_string(), "/abc/def/".to_string()).expect("not trailing slash");
    }
}
