
pub const PROJECT_VERSION_KEY: &str = "TRABAS_ROOT_VERSION";

pub fn set_root_version(version: &str) {
    std::env::set_var(PROJECT_VERSION_KEY, version);
}

pub fn get_root_version() -> String {
    std::env::var(PROJECT_VERSION_KEY)
        .unwrap_or_else(|_| "0.1.0".to_string())
}

pub fn validate_version(
    verions: String,
    against_version: String,
) -> bool {
    // validation based on valid semantic versioning
    // [Major].[Minor].[Patch]-[Pre-release].[Build metadata]
    // valid pre-releases for this project are:
    // - rc (release candidate)
    // - beta
    // - alpha
    // For example: 
    // 0.1.2 > 0.1.2-rc.1 > 0.1.2-rc.0 > 0.1.2-beta.1 > 0.1.2-alpha.1
    fn parse_version(ver: &str) -> (u32, u32, u32, Option<(String, u32)>) {
        let mut main = ver;
        let mut pre = None;
        if let Some(idx) = ver.find('-') {
            main = &ver[..idx];
            let pre_str = &ver[idx+1..];
            let mut parts = pre_str.split('.');
            let kind = parts.next().unwrap_or("").to_string();
            let num = parts.next().unwrap_or("0").parse().unwrap_or(0);
            pre = Some((kind, num));
        }

        let mut nums = main.split('.');
        let major = nums.next().unwrap_or("0").parse().unwrap_or(0);
        let minor = nums.next().unwrap_or("0").parse().unwrap_or(0);
        let patch = nums.next().unwrap_or("0").parse().unwrap_or(0);
        
        (major, minor, patch, pre)
    }

    let v1 = parse_version(&verions);
    let v2 = parse_version(&against_version);

    // compare major, minor, patch
    if v1.0 != v2.0 {
        return v1.0 > v2.0;
    }
    if v1.1 != v2.1 {
        return v1.1 > v2.1;
    }
    if v1.2 != v2.2 {
        return v1.2 > v2.2;
    }

    // pre-release ordering: None > rc > beta > alpha
    fn pre_order(pre: &Option<(String, u32)>) -> (i32, u32) {
        match pre {
            None => (3, 0),
            Some((kind, num)) => {
                let ord = match kind.as_str() {
                    "rc" => 2,
                    "beta" => 1,
                    "alpha" => 0,
                    _ => -1,
                };
                (ord, *num)
            }
        }
    }
    let p1 = pre_order(&v1.3);
    let p2 = pre_order(&v2.3);
    if p1.0 != p2.0 {
        return p1.0 > p2.0;
    }
    
    // lastly, compare the pre-release numbers
    p1.1 >= p2.1
}
