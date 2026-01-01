use crate::common::ToCase as _;
use crate::{common::fs_write, model_generator::filters, schema::_to_ident_name};
use anyhow::Result;
use askama::Template;
use regex::Regex;
use std::{fs, path::Path};

pub mod base_domain;
pub mod base_relations;
pub mod repositories;

pub fn write_repositories_rs(domain_src_dir: &Path, db: &str) -> Result<()> {
    let file_path = domain_src_dir.join("repository.rs");
    let mut content = if !file_path.exists() {
        #[derive(Template)]
        #[template(path = "domain/src/repository.rs", escape = "none")]
        pub struct DomainRepositoryTemplate;

        DomainRepositoryTemplate.render()?
    } else {
        fs::read_to_string(&file_path)?.replace("\r\n", "\n")
    };
    let db_snake = db.to_snake();
    let chk = format!(
        "\npub use repository_{} as {};\n",
        db_snake,
        _to_ident_name(&db_snake)
    );
    if !content.contains(&chk) {
        #[derive(Template)]
        #[template(
            source = r###"
pub use repository_@{ db|snake }@ as @{ db|snake|ident }@;
// Do not modify this line. (Mod)"###,
            ext = "txt",
            escape = "none"
        )]
        pub struct ModTemplate<'a> {
            pub db: &'a str,
        }

        let tpl = ModTemplate { db }.render()?;
        content = content.replace("// Do not modify this line. (Mod)", tpl.trim_start());

        #[derive(Template)]
        #[template(
            source = r###"
#[cfg(any(feature = "mock", test))]
use self::@{ db|snake|ident }@::@{ db|pascal }@Repository as _;
// Do not modify this line. (UseRepo)"###,
            ext = "txt",
            escape = "none"
        )]
        pub struct DomainModelsUseRepoTemplate<'a> {
            pub db: &'a str,
        }

        let re = Regex::new(r"// Do not modify this line\. \(UseRepo\)").unwrap();
        let tpl = DomainModelsUseRepoTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        #[derive(Template)]
        #[template(
            source = r###"
    fn @{ db|snake }@_repository(&self) -> Box<dyn @{ db|snake|ident }@::@{ db|pascal }@Repository> {
        unimplemented!("@{ db|snake }@_repository is unimplemented.")
    }
    fn @{ db|snake }@_query(&self) -> Box<dyn @{ db|snake|ident }@::@{ db|pascal }@QueryService> {
        unimplemented!("@{ db|snake }@_query is unimplemented.")
    }
    // Do not modify this line. (Repo)"###,
            ext = "txt",
            escape = "none"
        )]
        pub struct DomainModelsRepoTemplate<'a> {
            pub db: &'a str,
        }
        let re = Regex::new(r"// Do not modify this line\. \(Repo\)").unwrap();
        let tpl = DomainModelsRepoTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        #[derive(Template)]
        #[template(
            source = r###"
    pub @{ db|snake|ident }@: @{ db|snake|ident }@::Emu@{ db|pascal }@Repository,
    // Do not modify this line. (EmuRepo)"###,
            ext = "txt",
            escape = "none"
        )]
        pub struct DomainModelsEmuRepoTemplate<'a> {
            pub db: &'a str,
        }

        let re = Regex::new(r"// Do not modify this line\. \(EmuRepo\)").unwrap();
        let tpl = DomainModelsEmuRepoTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        #[derive(Template)]
        #[template(
            source = r###"
    fn @{ db|snake|ident }@_repository(&self) -> Box<dyn @{ db|snake|ident }@::@{ db|pascal }@Repository> {
        Box::new(self.@{ db|snake|ident }@.clone())
    }
    fn @{ db|snake|ident }@_query(&self) -> Box<dyn @{ db|snake|ident }@::@{ db|pascal }@QueryService> {
        Box::new(self.@{ db|snake|ident }@.clone())
    }
    // Do not modify this line. (EmuImpl)"###,
            ext = "txt",
            escape = "none"
        )]
        pub struct DomainModelsEmuImplTemplate<'a> {
            pub db: &'a str,
        }

        let re = Regex::new(r"// Do not modify this line\. \(EmuImpl\)").unwrap();
        let tpl = DomainModelsEmuImplTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        #[derive(Template)]
        #[template(
            source = r###"
        self.@{ db|snake|ident }@.begin().await?;
        // Do not modify this line. (EmuImplStart)"###,
            ext = "txt",
            escape = "none"
        )]
        pub struct DomainModelsEmuImplStartTemplate<'a> {
            pub db: &'a str,
        }

        let re = Regex::new(r"// Do not modify this line\. \(EmuImplStart\)").unwrap();
        let tpl = DomainModelsEmuImplStartTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        #[derive(Template)]
        #[template(
            source = r###"
        self.@{ db|snake|ident }@.commit().await?;
        // Do not modify this line. (EmuImplCommit)"###,
            ext = "txt",
            escape = "none"
        )]
        pub struct DomainModelsEmuImplCommitTemplate<'a> {
            pub db: &'a str,
        }

        let re = Regex::new(r"// Do not modify this line\. \(EmuImplCommit\)").unwrap();
        let tpl = DomainModelsEmuImplCommitTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();

        #[derive(Template)]
        #[template(
            source = r###"
        self.@{ db|snake|ident }@.rollback().await?;
        // Do not modify this line. (EmuImplRollback)"###,
            ext = "txt",
            escape = "none"
        )]
        pub struct DomainModelsEmuImplRollbackTemplate<'a> {
            pub db: &'a str,
        }

        let re = Regex::new(r"// Do not modify this line\. \(EmuImplRollback\)").unwrap();
        let tpl = DomainModelsEmuImplRollbackTemplate { db }.render()?;
        content = re.replace(&content, tpl.trim_start()).to_string();
    }

    fs_write(&file_path, &*content)?;
    Ok(())
}
