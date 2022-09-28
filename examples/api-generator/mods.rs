use quote::{format_ident, quote};
use std::{
    collections::{BTreeMap, VecDeque},
    ffi::OsString,
    fs::{create_dir_all, write, OpenOptions},
    io::{BufWriter, Result as IoResult, Write},
    path::Path,
};

type Tree = BTreeMap<OsString, DirOrFile>;

#[derive(Clone, Debug)]
enum DirOrFile {
    Dir(Box<Tree>),
    File,
}

#[derive(Default, Clone, Debug)]
pub(super) struct Mods {
    root: Tree,
}

impl Mods {
    pub(super) fn add(&mut self, base_name: OsString, namespace: VecDeque<OsString>) {
        return add(base_name, namespace, &mut self.root);

        fn add(base_name: OsString, mut namespace: VecDeque<OsString>, tree: &mut Tree) {
            if let Some(namespace_root) = namespace.pop_front() {
                let entry = tree
                    .entry(namespace_root)
                    .or_insert_with(|| DirOrFile::Dir(Box::new(Tree::new())));
                match entry {
                    DirOrFile::Dir(sub_tree) => add(base_name, namespace, sub_tree),
                    DirOrFile::File { .. } => unreachable!("Cannot insert entry into File"),
                };
            } else {
                tree.insert(base_name, DirOrFile::File);
            }
        }
    }

    pub(super) fn write_to_python_mod(&self, mod_name: &str, src_dir_path: &Path) -> IoResult<()> {
        return write_to_python_mod(src_dir_path, mod_name, &self.root);

        fn write_to_python_mod(dir_path: &Path, mod_name: &str, tree: &Tree) -> IoResult<()> {
            let mod_file_path = dir_path.join("mod.rs");
            let mut mods = Vec::new();
            for (mod_name, item) in tree.iter() {
                if let DirOrFile::Dir(subtree) = item {
                    let mod_dir_path = dir_path.join(mod_name);
                    write_to_python_mod(&mod_dir_path, mod_name.to_str().unwrap(), subtree)?;
                }

                let mod_name = format_ident!("r#{}", mod_name.to_str().unwrap());
                mods.push(mod_name);
            }

            let mods_declarations_token_stream = mods
                .iter()
                .map(|mod_name| quote! {mod #mod_name;})
                .collect::<Vec<_>>();
            let create_mods_token_streams = mods
                .iter()
                .map(|mod_name| quote! {m.add_submodule(#mod_name::create_module(py)?)?;})
                .collect::<Vec<_>>();
            let create_mods_token_stream = if create_mods_token_streams.is_empty() {
                None
            } else {
                Some(quote! {
                    pub(super) fn create_module(py: Python<'_>) -> PyResult<&PyModule> {
                        let m = PyModule::new(py, #mod_name)?;
                        #(#create_mods_token_streams)*
                        Ok(m)
                    }
                })
            };
            let token_streams = quote! {
                use pyo3::prelude::*;

                #(#mods_declarations_token_stream)*
                #create_mods_token_stream
            };
            create_dir_all(dir_path)?;
            let auto_generated_code =
                "// THIS FILE IS GENERATED BY api-generator, DO NOT EDIT DIRECTLY!\n//\n"
                    .to_owned()
                    + &token_streams.to_string();
            write(mod_file_path, auto_generated_code.as_bytes())?;
            Ok(())
        }
    }

    pub(super) fn write_sphinx_index(&self, mod_name: &str, src_dir_path: &Path) -> IoResult<()> {
        return write_sphinx_index(src_dir_path, mod_name, &["qiniu_bindings"], &self.root);

        fn write_sphinx_index(
            dir_path: &Path,
            mod_name: &str,
            module_path_segments: &[&str],
            tree: &Tree,
        ) -> IoResult<()> {
            let index_file_path = dir_path.join("index.rst");
            let new_module_path_segments = {
                let mut module_path_segments = module_path_segments.to_owned();
                module_path_segments.push(mod_name);
                module_path_segments
            };
            let mut mods = Vec::new();
            for (mod_name, item) in tree.iter() {
                if let DirOrFile::Dir(subtree) = item {
                    let mod_dir_path = dir_path.join(mod_name);
                    write_sphinx_index(
                        &mod_dir_path,
                        mod_name.to_str().unwrap(),
                        &new_module_path_segments,
                        subtree,
                    )?;
                }
                let mut segments = new_module_path_segments.to_owned();
                segments.push(mod_name.to_str().unwrap());
                mods.push((segments, matches!(item, DirOrFile::Dir(_))));
            }
            return write_to_index_rst(&index_file_path, &mods);

            fn write_to_index_rst<M: AsRef<str>>(
                path: impl AsRef<Path>,
                mods: &[(Vec<M>, bool)],
            ) -> IoResult<()> {
                let mut file = BufWriter::new(
                    OpenOptions::new()
                        .create(true)
                        .write(true)
                        .truncate(true)
                        .open(path)?,
                );
                for (full_module_segments, _) in mods {
                    let full_segments = full_module_segments
                        .iter()
                        .map(|s| s.as_ref())
                        .collect::<Vec<_>>();
                    write_one_mod_to_index_rst(&mut file, full_segments.join("."))?;
                }
                for (full_module_segments, is_dir) in mods {
                    if *is_dir {
                        let full_segments = full_module_segments
                            .iter()
                            .skip(1)
                            .map(|s| s.as_ref())
                            .collect::<Vec<_>>();
                        write_dir_include_to_index_rst(&mut file, full_segments.join("/"))?;
                    }
                }
                file.flush()
            }

            fn write_one_mod_to_index_rst(
                file: &mut dyn Write,
                module: impl AsRef<str>,
            ) -> IoResult<()> {
                writeln!(file, ".. automodule:: {}", module.as_ref())?;
                writeln!(file, "  :members:")?;
                writeln!(file, "  :show-inheritance:")?;
                writeln!(file, "  :noindex:")?;
                writeln!(file)?;
                Ok(())
            }

            fn write_dir_include_to_index_rst(
                file: &mut dyn Write,
                dir: impl AsRef<str>,
            ) -> IoResult<()> {
                writeln!(file, ".. include:: ../src/{}/index.rst", dir.as_ref())?;
                Ok(())
            }
        }
    }
}
