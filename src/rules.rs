//! This module provides functions to manipulate debian/rules file.

use makefile_lossless::{Makefile, Rule};

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

/// Discard a pointless override rule from a Makefile.
///
/// A pointless override is one that just calls the base command without any modifications.
/// For example:
/// ```makefile
/// override_dh_auto_build:
///     dh_auto_build
/// ```
///
/// Note: The makefile-lossless crate's `recipes()` method only returns actual command lines,
/// not comment lines, so comment lines are automatically ignored.
///
/// # Arguments
/// * `makefile` - The makefile to modify
/// * `rule` - The rule to check and potentially remove
///
/// # Returns
/// `true` if the rule was removed, `false` otherwise
pub fn discard_pointless_override(makefile: &mut Makefile, rule: &Rule) -> bool {
    // Get the targets for this rule
    let targets: Vec<String> = rule.targets().collect();

    // Check if any target starts with "override_"
    let override_target = targets.iter().find(|t| t.starts_with("override_"));

    let Some(target) = override_target else {
        return false;
    };

    // Get the command name (strip "override_" prefix)
    let command = &target["override_".len()..];

    // Get the recipes (commands) for this rule
    // Note: recipes() only returns actual command lines, not comments
    let recipes: Vec<String> = rule.recipes().collect();

    // Filter out empty lines
    let effective_recipes: Vec<&String> = recipes
        .iter()
        .filter(|line| !line.trim().is_empty())
        .collect();

    // Check if there's exactly one effective recipe and it matches the command
    if effective_recipes.len() != 1 {
        return false;
    }

    let recipe = effective_recipes[0].trim();
    if recipe != command {
        return false;
    }

    // Check if there are any prerequisites
    let prereqs: Vec<String> = rule.prerequisites().collect();
    if !prereqs.is_empty() {
        return false;
    }

    // Remove the rule
    let rules: Vec<Rule> = makefile.rules().collect();
    for (i, r) in rules.iter().enumerate() {
        if r.targets().collect::<Vec<_>>() == targets {
            if makefile.remove_rule(i).is_ok() {
                // Also remove from .PHONY if present
                let _ = makefile.remove_phony_target(target);
                return true;
            }
        }
    }

    false
}

/// Discard all pointless override rules from a Makefile.
///
/// # Arguments
/// * `makefile` - The makefile to modify
///
/// # Returns
/// The number of rules that were removed
pub fn discard_pointless_overrides(makefile: &mut Makefile) -> usize {
    let mut removed = 0;

    // Collect all rules first to avoid modifying while iterating
    let rules: Vec<Rule> = makefile.rules().collect();

    for rule in rules {
        if discard_pointless_override(makefile, &rule) {
            removed += 1;
        }
    }

    removed
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

    #[test]
    fn test_discard_pointless_override() {
        // Test a pointless override that should be removed
        let makefile_text = r#"
override_dh_auto_build:
	dh_auto_build
"#;
        let mut makefile = makefile_text.parse::<Makefile>().unwrap();
        let rules: Vec<Rule> = makefile.rules().collect();
        assert_eq!(rules.len(), 1);

        let removed = discard_pointless_override(&mut makefile, &rules[0]);
        assert!(removed, "Should have removed the pointless override");

        let remaining_rules: Vec<Rule> = makefile.rules().collect();
        assert_eq!(remaining_rules.len(), 0, "Rule should be removed");
    }

    #[test]
    fn test_discard_pointless_override_with_args() {
        // Test an override with arguments - should NOT be removed
        let makefile_text = r#"
override_dh_auto_build:
	dh_auto_build --foo
"#;
        let mut makefile = makefile_text.parse::<Makefile>().unwrap();
        let rules: Vec<Rule> = makefile.rules().collect();
        assert_eq!(rules.len(), 1);

        let removed = discard_pointless_override(&mut makefile, &rules[0]);
        assert!(!removed, "Should NOT remove override with arguments");

        let remaining_rules: Vec<Rule> = makefile.rules().collect();
        assert_eq!(remaining_rules.len(), 1, "Rule should remain");
    }

    #[test]
    fn test_discard_pointless_override_with_comment() {
        // Test an override with just a comment - since recipes() doesn't return comments,
        // this should still be removed because only the actual command matters
        let makefile_text = r#"
override_dh_auto_build:
	# This is a comment
	dh_auto_build
"#;
        let mut makefile = makefile_text.parse::<Makefile>().unwrap();
        let rules: Vec<Rule> = makefile.rules().collect();
        assert_eq!(rules.len(), 1);

        // The recipes() method doesn't return comment lines, so this is still pointless
        let removed = discard_pointless_override(&mut makefile, &rules[0]);
        assert!(
            removed,
            "Should remove - recipes() doesn't include comments"
        );
    }

    #[test]
    fn test_discard_pointless_override_not_override() {
        // Test a regular rule that doesn't start with override_
        let makefile_text = r#"
build:
	dh_auto_build
"#;
        let mut makefile = makefile_text.parse::<Makefile>().unwrap();
        let rules: Vec<Rule> = makefile.rules().collect();
        assert_eq!(rules.len(), 1);

        let removed = discard_pointless_override(&mut makefile, &rules[0]);
        assert!(!removed, "Should NOT remove non-override rules");
    }

    #[test]
    fn test_discard_pointless_overrides() {
        // Test removing multiple pointless overrides
        let makefile_text = r#"
override_dh_auto_build:
	dh_auto_build

override_dh_auto_test:
	dh_auto_test

override_dh_auto_install:
	dh_auto_install --foo
"#;
        let mut makefile = makefile_text.parse::<Makefile>().unwrap();
        let initial_rules = makefile.rules().count();
        assert_eq!(initial_rules, 3);

        let removed = discard_pointless_overrides(&mut makefile);
        assert_eq!(removed, 2, "Should remove 2 pointless overrides");

        let remaining_rules: Vec<Rule> = makefile.rules().collect();
        assert_eq!(remaining_rules.len(), 1, "Should have 1 rule remaining");

        // Verify the remaining rule is the one with arguments
        let targets: Vec<String> = remaining_rules[0].targets().collect();
        assert_eq!(targets, vec!["override_dh_auto_install"]);
    }
}
