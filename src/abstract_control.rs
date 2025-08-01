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
}
