use std::str::FromStr;

fn main() {
    let s1 = "<@123456>";
    let s2 = "<@!123456>";
    let s3 = "123456";
    // Using simple regex/manual parse
    println!("s1: {:?}", parse_target(s1));
    println!("s2: {:?}", parse_target(s2));
    println!("s3: {:?}", parse_target(s3));
}

fn parse_target(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Ok(id) = s.parse::<u64>() {
        return Some(id);
    }
    if s.starts_with("<@") && s.ends_with('>') {
        let s = s.trim_start_matches("<@").trim_start_matches('!').trim_end_matches('>');
        if let Ok(id) = s.parse::<u64>() {
            return Some(id);
        }
    }
    None
}
