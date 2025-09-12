//! Abstract interface for editing debian packages, whether backed by real control files or
//! debcargo files.
use crate::relations::ensure_relation;
use debian_control::lossless::relations::{Entry, Relations};
use std::path::Path;

/// Interface for editing debian packages, whether backed by real control files or debcargo files.
pub trait AbstractControlEditor {
    /// Get the source package.
    fn source<'a>(&'a mut self) -> Option<Box<dyn AbstractSource<'a> + 'a>>;

    /// Get the binary packages.
    fn binaries<'a>(&'a mut self) -> Vec<Box<dyn AbstractBinary + 'a>>;

    /// Commit the changes.
    fn commit(&self) -> bool;

    /// Wrap and sort the control file.
    fn wrap_and_sort(&mut self);
}

/// An abstract source package.
pub trait AbstractSource<'a> {
    /// Get the name of the source package.
    fn name(&self) -> Option<String>;

    /// Ensure that a build dependency is present.
    fn ensure_build_dep(&mut self, dep: Entry);

    /// Set the maintainer of the source package.
    fn set_maintainer(&mut self, maintainer: &str);

    /// Set the uploaders of the source package.
    fn set_uploaders(&mut self, uploaders: &[&str]);

    /// Set the VCS URL for the source package.
    fn set_vcs_url(&mut self, vcs_type: &str, url: &str);

    /// Get the VCS URL for the source package.
    fn get_vcs_url(&self, vcs_type: &str) -> Option<String>;
}

/// An abstract binary package.
pub trait AbstractBinary {
    /// Get the name of the binary package.
    fn name(&self) -> Option<String>;
}

use crate::debcargo::{DebcargoBinary, DebcargoEditor, DebcargoSource};
use debian_control::{Binary as PlainBinary, Control as PlainControl, Source as PlainSource};

impl AbstractControlEditor for DebcargoEditor {
    fn source<'a>(&'a mut self) -> Option<Box<dyn AbstractSource<'a> + 'a>> {
        Some(Box::new(DebcargoEditor::source(self)) as Box<dyn AbstractSource<'a>>)
    }

    fn binaries<'a>(&'a mut self) -> Vec<Box<dyn AbstractBinary + 'a>> {
        DebcargoEditor::binaries(self)
            .map(|b| Box::new(b) as Box<dyn AbstractBinary>)
            .collect()
    }

    fn commit(&self) -> bool {
        DebcargoEditor::commit(self).unwrap()
    }

    fn wrap_and_sort(&mut self) {}
}

impl AbstractBinary for PlainBinary {
    fn name(&self) -> Option<String> {
        self.name()
    }
}

impl AbstractSource<'_> for PlainSource {
    fn name(&self) -> Option<String> {
        self.name()
    }

    fn ensure_build_dep(&mut self, dep: Entry) {
        if let Some(mut build_deps) = self.build_depends() {
            ensure_relation(&mut build_deps, dep);
            self.set_build_depends(&build_deps);
        } else {
            self.set_build_depends(&Relations::from(vec![dep]));
        }
    }

    fn set_maintainer(&mut self, maintainer: &str) {
        (self as &mut debian_control::lossless::Source).set_maintainer(maintainer);
    }

    fn set_uploaders(&mut self, uploaders: &[&str]) {
        (self as &mut debian_control::lossless::Source).set_uploaders(uploaders);
    }

    fn set_vcs_url(&mut self, vcs_type: &str, url: &str) {
        let field_name = format!("Vcs-{}", vcs_type);
        self.as_mut_deb822().set(&field_name, url);
    }

    fn get_vcs_url(&self, vcs_type: &str) -> Option<String> {
        let field_name = format!("Vcs-{}", vcs_type);
        self.as_deb822().get(&field_name)
    }
}

impl AbstractBinary for DebcargoBinary<'_> {
    fn name(&self) -> Option<String> {
        Some(self.name().to_string())
    }
}

impl<'a> AbstractSource<'a> for DebcargoSource<'a> {
    fn name(&self) -> Option<String> {
        self.name()
    }

    fn ensure_build_dep(&mut self, dep: Entry) {
        // TODO: Check that it's not already there
        if let Some(build_deps) = self
            .toml_section_mut()
            .get_mut("build_depends")
            .and_then(|v| v.as_array_mut())
        {
            build_deps.push(dep.to_string());
        }
    }

    fn set_maintainer(&mut self, maintainer: &str) {
        (self as &mut crate::debcargo::DebcargoSource).set_maintainer(maintainer);
    }

    fn set_uploaders(&mut self, uploaders: &[&str]) {
        (self as &mut crate::debcargo::DebcargoSource)
            .set_uploaders(uploaders.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    }

    fn set_vcs_url(&mut self, vcs_type: &str, url: &str) {
        (self as &mut crate::debcargo::DebcargoSource).set_vcs_url(vcs_type, url);
    }

    fn get_vcs_url(&self, vcs_type: &str) -> Option<String> {
        match vcs_type.to_lowercase().as_str() {
            "git" => self.vcs_git(),
            "browser" => self.vcs_browser(),
            _ => self.get_extra_field(&format!("Vcs-{}", vcs_type)),
        }
    }
}

impl<E: crate::editor::Editor<PlainControl>> AbstractControlEditor for E {
    fn source<'a>(&'a mut self) -> Option<Box<dyn AbstractSource<'a> + 'a>> {
        PlainControl::source(self).map(|s| Box::new(s) as Box<dyn AbstractSource>)
    }

    fn binaries<'a>(&'a mut self) -> Vec<Box<dyn AbstractBinary + 'a>> {
        PlainControl::binaries(self)
            .map(|b| Box::new(b) as Box<dyn AbstractBinary>)
            .collect()
    }

    fn commit(&self) -> bool {
        !(self as &dyn crate::editor::Editor<PlainControl>)
            .commit()
            .unwrap()
            .is_empty()
    }

    fn wrap_and_sort(&mut self) {
        (self as &mut dyn crate::editor::Editor<PlainControl>).wrap_and_sort(
            deb822_lossless::Indentation::Spaces(4),
            false,
            None,
        )
    }
}

/// Open a control file for editing.
pub fn edit_control<'a>(
    tree: &dyn breezyshim::workingtree::WorkingTree,
    subpath: &Path,
) -> Result<Box<dyn AbstractControlEditor + 'a>, crate::editor::EditorError> {
    if tree.has_filename(&subpath.join("debian/debcargo.toml")) {
        Ok(Box::new(crate::debcargo::DebcargoEditor::from_directory(
            &tree.abspath(subpath).unwrap(),
        )?))
    } else {
        let control_path = tree.abspath(&subpath.join(std::path::Path::new("debian/control")));
        Ok(Box::new(crate::control::TemplatedControlEditor::open(
            control_path.unwrap(),
        )?) as Box<dyn AbstractControlEditor>)
    }
}

#[cfg(test)]
mod tests {
    use breezyshim::controldir::{create_standalone_workingtree, ControlDirFormat};
    use breezyshim::prelude::*;
    use std::path::Path;
    use std::str::FromStr;

    #[test]
    fn test_edit_control_debcargo() {
        let td = tempfile::tempdir().unwrap();
        let tree = create_standalone_workingtree(td.path(), &ControlDirFormat::default()).unwrap();
        // Write dummy debcargo.toml
        tree.mkdir(Path::new("debian")).unwrap();
        std::fs::write(
            td.path().join("debian/debcargo.toml"),
            br#"
maintainer = "Alice <alice@example.com>"
homepage = "https://example.com"
description = "Example package"
"#,
        )
        .unwrap();

        std::fs::write(
            td.path().join("Cargo.toml"),
            br#"
[package]
name = "example"
version = "0.1.0"
edition = "2018"
"#,
        )
        .unwrap();

        tree.add(&[(Path::new("debian")), (Path::new("debian/debcargo.toml"))])
            .unwrap();

        let editor = super::edit_control(&tree, Path::new("")).unwrap();

        editor.commit();
    }

    #[test]
    fn test_edit_control_regular() {
        let td = tempfile::tempdir().unwrap();
        let tree = create_standalone_workingtree(td.path(), &ControlDirFormat::default()).unwrap();
        // Write dummy debian/control
        tree.mkdir(Path::new("debian")).unwrap();
        tree.put_file_bytes_non_atomic(
            Path::new("debian/control"),
            br#"
Source: example
Maintainer: Alice <alice@example.com>
Homepage: https://example.com

Package: example
Architecture: any
Description: Example package
"#,
        )
        .unwrap();

        tree.add(&[(Path::new("debian")), (Path::new("debian/control"))])
            .unwrap();

        let editor = super::edit_control(&tree, Path::new("")).unwrap();

        editor.commit();
    }

    #[test]
    fn test_edit_source_ensure_build_depends() {
        let td = tempfile::tempdir().unwrap();
        let tree = create_standalone_workingtree(td.path(), &ControlDirFormat::default()).unwrap();
        // Write dummy debian/control
        tree.mkdir(Path::new("debian")).unwrap();
        tree.put_file_bytes_non_atomic(
            Path::new("debian/control"),
            br#"
Source: example
Maintainer: Alice <alice@example.com>
Build-Depends: libc6

Package: example
Architecture: any
Description: Example package
"#,
        )
        .unwrap();
        tree.add(&[Path::new("debian/control")]).unwrap();

        let mut editor = super::edit_control(&tree, Path::new("")).unwrap();
        let mut source = editor.source().unwrap();
        source.ensure_build_dep(
            debian_control::lossless::relations::Entry::from_str("libssl-dev").unwrap(),
        );
        std::mem::drop(source);
        editor.commit();

        let text = tree.get_file_text(Path::new("debian/control")).unwrap();
        assert_eq!(
            std::str::from_utf8(&text).unwrap(),
            r#"
Source: example
Maintainer: Alice <alice@example.com>
Build-Depends: libc6, libssl-dev

Package: example
Architecture: any
Description: Example package
"#
        );
    }

    #[test]
    fn test_abstract_source_set_vcs_url_plain() {
        let td = tempfile::tempdir().unwrap();
        let tree = create_standalone_workingtree(td.path(), &ControlDirFormat::default()).unwrap();
        // Write dummy debian/control
        tree.mkdir(Path::new("debian")).unwrap();
        tree.put_file_bytes_non_atomic(
            Path::new("debian/control"),
            br#"Source: example
Maintainer: Alice <alice@example.com>

Package: example
Architecture: any
Description: Example package
"#,
        )
        .unwrap();
        tree.add(&[Path::new("debian/control")]).unwrap();

        let mut editor = super::edit_control(&tree, Path::new("")).unwrap();
        let mut source = editor.source().unwrap();

        // Test setting various VCS URLs
        source.set_vcs_url("Git", "https://github.com/example/repo.git");
        source.set_vcs_url("Browser", "https://github.com/example/repo");

        std::mem::drop(source);
        editor.commit();

        let text = tree.get_file_text(Path::new("debian/control")).unwrap();
        assert_eq!(
            std::str::from_utf8(&text).unwrap(),
            r#"Source: example
Maintainer: Alice <alice@example.com>
Vcs-Git: https://github.com/example/repo.git
Vcs-Browser: https://github.com/example/repo

Package: example
Architecture: any
Description: Example package
"#
        );
    }

    #[test]
    fn test_abstract_source_set_vcs_url_debcargo() {
        let td = tempfile::tempdir().unwrap();
        let tree = create_standalone_workingtree(td.path(), &ControlDirFormat::default()).unwrap();
        // Write dummy debcargo.toml
        tree.mkdir(Path::new("debian")).unwrap();
        std::fs::write(
            td.path().join("debian/debcargo.toml"),
            br#"maintainer = "Alice <alice@example.com>"
"#,
        )
        .unwrap();

        std::fs::write(
            td.path().join("Cargo.toml"),
            br#"[package]
name = "example"
version = "0.1.0"
"#,
        )
        .unwrap();

        tree.add(&[(Path::new("debian")), (Path::new("debian/debcargo.toml"))])
            .unwrap();

        let mut editor = super::edit_control(&tree, Path::new("")).unwrap();
        let mut source = editor.source().unwrap();

        // Test setting native VCS URLs
        source.set_vcs_url("Git", "https://github.com/example/repo.git");
        source.set_vcs_url("Browser", "https://github.com/example/repo");

        // Test setting non-native VCS URL
        source.set_vcs_url("Svn", "https://svn.example.com/repo");

        std::mem::drop(source);
        editor.commit();

        // Read back the debcargo.toml to verify
        let content = std::fs::read_to_string(td.path().join("debian/debcargo.toml")).unwrap();
        assert_eq!(
            content,
            r#"maintainer = "Alice <alice@example.com>"

[source]
vcs_git = "https://github.com/example/repo.git"
vcs_browser = "https://github.com/example/repo"
extra_lines = ["Vcs-Svn: https://svn.example.com/repo"]
"#
        );
    }

    #[test]
    fn test_abstract_source_get_vcs_url_plain() {
        let td = tempfile::tempdir().unwrap();
        let tree = create_standalone_workingtree(td.path(), &ControlDirFormat::default()).unwrap();
        // Write dummy debian/control with VCS fields
        tree.mkdir(Path::new("debian")).unwrap();
        tree.put_file_bytes_non_atomic(
            Path::new("debian/control"),
            br#"Source: example
Maintainer: Alice <alice@example.com>
Vcs-Git: https://github.com/example/repo.git
Vcs-Browser: https://github.com/example/repo
Vcs-Svn: https://svn.example.com/repo

Package: example
Architecture: any
Description: Example package
"#,
        )
        .unwrap();
        tree.add(&[Path::new("debian/control")]).unwrap();

        let mut editor = super::edit_control(&tree, Path::new("")).unwrap();
        let source = editor.source().unwrap();

        // Test getting various VCS URLs
        assert_eq!(
            source.get_vcs_url("Git"),
            Some("https://github.com/example/repo.git".to_string())
        );
        assert_eq!(
            source.get_vcs_url("Browser"),
            Some("https://github.com/example/repo".to_string())
        );
        assert_eq!(
            source.get_vcs_url("Svn"),
            Some("https://svn.example.com/repo".to_string())
        );
        assert_eq!(source.get_vcs_url("Bzr"), None);
    }

    #[test]
    fn test_abstract_source_get_vcs_url_debcargo() {
        let td = tempfile::tempdir().unwrap();
        let tree = create_standalone_workingtree(td.path(), &ControlDirFormat::default()).unwrap();
        // Write dummy debcargo.toml with VCS fields
        tree.mkdir(Path::new("debian")).unwrap();
        std::fs::write(
            td.path().join("debian/debcargo.toml"),
            br#"maintainer = "Alice <alice@example.com>"

[source]
vcs_git = "https://github.com/example/repo.git"
vcs_browser = "https://github.com/example/repo"
extra_lines = ["Vcs-Svn: https://svn.example.com/repo", "Vcs-Bzr: https://bzr.example.com/repo"]
"#,
        )
        .unwrap();

        std::fs::write(
            td.path().join("Cargo.toml"),
            br#"[package]
name = "example"
version = "0.1.0"
"#,
        )
        .unwrap();

        tree.add(&[(Path::new("debian")), (Path::new("debian/debcargo.toml"))])
            .unwrap();

        let mut editor = super::edit_control(&tree, Path::new("")).unwrap();
        let source = editor.source().unwrap();

        // Test getting native VCS URLs
        assert_eq!(
            source.get_vcs_url("Git"),
            Some("https://github.com/example/repo.git".to_string())
        );
        assert_eq!(
            source.get_vcs_url("Browser"),
            Some("https://github.com/example/repo".to_string())
        );

        // Test getting non-native VCS URLs from extra_lines
        assert_eq!(
            source.get_vcs_url("Svn"),
            Some("https://svn.example.com/repo".to_string())
        );
        assert_eq!(
            source.get_vcs_url("Bzr"),
            Some("https://bzr.example.com/repo".to_string())
        );

        // Test getting non-existent VCS URL
        assert_eq!(source.get_vcs_url("Hg"), None);
    }
}
