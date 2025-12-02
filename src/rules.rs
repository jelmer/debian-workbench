//! This module provides functions to manipulate debian/rules file.

/// Add a particular value to a with argument.
pub fn dh_invoke_add_with(line: &str, with_argument: &str) -> String {
    if line.contains(with_argument) {
        return line.to_owned();
    }
    if !line.contains(" --with") {
        return format!("{} --with={}", line, with_argument);
    }

    lazy_regex::regex_replace!(
        r"([ \t])--with([ =])([^ \t]+)",
        line,
        |_, head, _with, tail| format!("{}--with={},{}", head, with_argument, tail)
    )
    .to_string()
}

/// Obtain the value of a with argument.
pub fn dh_invoke_get_with(line: &str) -> Vec<String> {
    let mut ret = Vec::new();
    for cap in lazy_regex::regex!("[ \t]--with[ =]([^ \t]+)").captures_iter(line) {
        if let Some(m) = cap.get(1) {
            ret.extend(m.as_str().split(',').map(|s| s.to_owned()));
        }
    }
    ret
}

/// Drop a particular value from a with argument.
///
/// # Arguments
/// * `line` - The command line to modify
/// * `with_argument` - The with argument to remove
///
/// # Returns
/// The modified line with the argument removed
///
/// # Examples
/// ```rust
/// use debian_analyzer::rules::dh_invoke_drop_with;
/// assert_eq!(
///     dh_invoke_drop_with("dh $@ --with=foo,bar", "foo"),
///     "dh $@ --with=bar"
/// );
/// assert_eq!(
///     dh_invoke_drop_with("dh $@ --with=foo", "foo"),
///     "dh $@"
/// );
/// ```
pub fn dh_invoke_drop_with(line: &str, with_argument: &str) -> String {
    if !line.contains(with_argument) {
        return line.to_owned();
    }

    let mut result = line.to_owned();
    let escaped = regex::escape(with_argument);

    // It's the only with argument
    if let Ok(re) = regex::Regex::new(&format!(r"[ \t]--with[ =]{}( .+|)$", escaped)) {
        result = re.replace(&result, "$1").to_string();
    }

    // It's at the beginning
    if let Ok(re) = regex::Regex::new(&format!(r"([ \t])--with([ =]){},", escaped)) {
        result = re.replace(&result, "${1}--with${2}").to_string();
    }

    // It's in the middle or end
    if let Ok(re) = regex::Regex::new(&format!(r"([ \t])--with([ =])(.+),{}([ ,])", escaped)) {
        result = re.replace(&result, "${1}--with${2}${3}${4}").to_string();
    }

    // It's at the end
    if let Ok(re) = regex::Regex::new(&format!(r"([ \t])--with([ =])(.+),{}$", escaped)) {
        result = re.replace(&result, "${1}--with${2}${3}").to_string();
    }

    result
}

/// Drop a particular argument from a dh invocation.
///
/// # Arguments
/// * `line` - The command line to modify
/// * `argument` - The argument to remove
///
/// # Returns
/// The modified line with the argument removed
///
/// # Examples
/// ```rust
/// use debian_analyzer::rules::dh_invoke_drop_argument;
/// assert_eq!(
///     dh_invoke_drop_argument("dh $@ --foo --bar", "--foo"),
///     "dh $@ --bar"
/// );
/// ```
pub fn dh_invoke_drop_argument(line: &str, argument: &str) -> String {
    if !line.contains(argument) {
        return line.to_owned();
    }

    let mut result = line.to_owned();
    let escaped = regex::escape(argument);

    // At the end
    if let Ok(re) = regex::Regex::new(&format!(r"[ \t]+{}$", escaped)) {
        result = re.replace(&result, "").to_string();
    }

    // In the middle
    if let Ok(re) = regex::Regex::new(&format!(r"([ \t]){}[ \t]", escaped)) {
        result = re.replace(&result, "$1").to_string();
    }

    result
}

/// Replace one argument with another in a dh invocation.
///
/// # Arguments
/// * `line` - The command line to modify
/// * `old` - The argument to replace
/// * `new` - The new argument value
///
/// # Returns
/// The modified line with the argument replaced
///
/// # Examples
/// ```rust
/// use debian_analyzer::rules::dh_invoke_replace_argument;
/// assert_eq!(
///     dh_invoke_replace_argument("dh $@ --foo", "--foo", "--bar"),
///     "dh $@ --bar"
/// );
/// ```
pub fn dh_invoke_replace_argument(line: &str, old: &str, new: &str) -> String {
    if !line.contains(old) {
        return line.to_owned();
    }

    let mut result = line.to_owned();
    let escaped = regex::escape(old);

    // At the end
    if let Ok(re) = regex::Regex::new(&format!(r"([ \t]){}$", escaped)) {
        result = re.replace(&result, format!("$1{}", new)).to_string();
    }

    // In the middle
    if let Ok(re) = regex::Regex::new(&format!(r"([ \t]){}([ \t])", escaped)) {
        result = re.replace(&result, format!("$1{}$2", new)).to_string();
    }

    result
}

/// Check if a debian/rules file uses CDBS.
///
/// # Arguments
/// * `path` - Path to the debian/rules file
///
/// # Returns
/// true if the file includes CDBS, false otherwise
///
/// # Examples
/// ```rust,no_run
/// use debian_analyzer::rules::check_cdbs;
/// use std::path::Path;
/// assert!(!check_cdbs(Path::new("debian/rules")));
/// ```
pub fn check_cdbs(path: &std::path::Path) -> bool {
    let Ok(contents) = std::fs::read(path) else {
        return false;
    };

    for line in contents.split(|&b| b == b'\n') {
        let trimmed = line.strip_prefix(b"-").unwrap_or(line);
        if trimmed.starts_with(b"include /usr/share/cdbs/") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dh_invoke_add_with() {
        assert_eq!(dh_invoke_add_with("dh", "blah"), "dh --with=blah");
        assert_eq!(
            dh_invoke_add_with("dh --with=foo", "blah"),
            "dh --with=blah,foo"
        );
        assert_eq!(
            dh_invoke_add_with("dh --with=foo --other", "blah"),
            "dh --with=blah,foo --other"
        );
    }

    #[test]
    fn test_dh_invoke_get_with() {
        assert_eq!(dh_invoke_get_with("dh --with=blah --foo"), vec!["blah"]);
        assert_eq!(dh_invoke_get_with("dh --with=blah"), vec!["blah"]);
        assert_eq!(
            dh_invoke_get_with("dh --with=blah,blie"),
            vec!["blah", "blie"]
        );
    }

    #[test]
    fn test_dh_invoke_drop_with() {
        assert_eq!(dh_invoke_drop_with("dh --with=blah", "blah"), "dh");
        assert_eq!(
            dh_invoke_drop_with("dh --with=blah,foo", "blah"),
            "dh --with=foo"
        );
        assert_eq!(
            dh_invoke_drop_with("dh --with=blah,foo --other", "blah"),
            "dh --with=foo --other"
        );
        assert_eq!(dh_invoke_drop_with("dh --with=blah", "blah"), "dh");
        assert_eq!(
            dh_invoke_drop_with("dh --with=foo,blah", "blah"),
            "dh --with=foo"
        );
        assert_eq!(
            dh_invoke_drop_with(
                "dh $@ --verbose --with autoreconf,systemd,cme-upgrade",
                "systemd"
            ),
            "dh $@ --verbose --with autoreconf,cme-upgrade"
        );
        assert_eq!(
            dh_invoke_drop_with(
                "dh $@ --with gir,python3,sphinxdoc,systemd --without autoreconf --buildsystem=cmake",
                "systemd"
            ),
            "dh $@ --with gir,python3,sphinxdoc --without autoreconf --buildsystem=cmake"
        );
        assert_eq!(
            dh_invoke_drop_with("dh $@ --with systemd", "systemd"),
            "dh $@"
        );
    }

    #[test]
    fn test_dh_invoke_drop_argument() {
        assert_eq!(
            dh_invoke_drop_argument("dh $@ --foo --bar", "--foo"),
            "dh $@ --bar"
        );
        assert_eq!(
            dh_invoke_drop_argument("dh $@ --foo --bar", "--bar"),
            "dh $@ --foo"
        );
        assert_eq!(dh_invoke_drop_argument("dh $@ --foo", "--foo"), "dh $@");
    }

    #[test]
    fn test_dh_invoke_replace_argument() {
        assert_eq!(
            dh_invoke_replace_argument("dh $@ --foo", "--foo", "--bar"),
            "dh $@ --bar"
        );
        assert_eq!(
            dh_invoke_replace_argument("dh $@ --foo --baz", "--foo", "--bar"),
            "dh $@ --bar --baz"
        );
    }
}
