//! Compatibility ports for the independently maintained public pack pipeline.
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use velora_shared::Loader;
use serde::Deserialize;
use velora_panel_packs as owned;
pub use owned::{NewFile, MrVersion, MrVersionFile, files_url, detect_root};

#[derive(Debug, Clone, Default)]
pub struct PackInfo {pub mc_version:String,pub loader:Loader,pub loader_version:Option<String>,pub name:String,pub version:String,pub files:Vec<NewFile>}
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Fallback {pub mc_version:Option<String>,pub loader:Option<Loader>,pub loader_version:Option<String>}
fn public_loader(loader:Loader)->owned::Loader{match loader{Loader::Vanilla=>owned::Loader::Vanilla,Loader::Fabric=>owned::Loader::Fabric,Loader::Quilt=>owned::Loader::Quilt,Loader::Forge=>owned::Loader::Forge,Loader::NeoForge=>owned::Loader::NeoForge}}
fn host_loader(loader:owned::Loader)->Loader{match loader{owned::Loader::Vanilla=>Loader::Vanilla,owned::Loader::Fabric=>Loader::Fabric,owned::Loader::Quilt=>Loader::Quilt,owned::Loader::Forge=>Loader::Forge,owned::Loader::NeoForge=>Loader::NeoForge}}
fn host_error(error:owned::AppError)->AppError{AppError{status:axum::http::StatusCode::from_u16(error.status).unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR),message:error.message}}
struct Context<'a>(&'a AppState);
impl owned::PackHost for Context<'_>{
 fn http(&self)->&reqwest::Client{&self.0.http}
 fn files_dir(&self)->std::path::PathBuf{self.0.cfg.files_dir()}
 fn write_pool(&self)->&sqlx::SqlitePool{&self.0.db}
 fn platform_pool(&self)->&sqlx::SqlitePool{&self.0.platform_db}
 async fn curseforge_key(&self)->owned::AppResult<String>{crate::store::curseforge_key(self.0).await.map_err(|e|owned::AppError{status:e.status.as_u16(),message:e.message})}
 fn now(&self)->String{crate::db::now()}
}
fn into_host(info:owned::PackInfo)->PackInfo{PackInfo{mc_version:info.mc_version,loader:host_loader(info.loader),loader_version:info.loader_version,name:info.name,version:info.version,files:info.files}}
pub async fn fetch_modrinth(state:&AppState,version_id:&str)->AppResult<(Vec<u8>,String,Option<String>)>{owned::fetch_modrinth(&Context(state),version_id).await.map_err(host_error)}
pub async fn fetch_curseforge(state:&AppState,mod_id:i64,file_id:i64)->AppResult<(Vec<u8>,String,Option<String>)>{owned::fetch_curseforge(&Context(state),mod_id,file_id).await.map_err(host_error)}
pub async fn cf_raw_get(state:&AppState,path:&str)->AppResult<serde_json::Value>{owned::cf_raw_get(&Context(state),path).await.map_err(host_error)}
pub fn parse_cf_loader(mc:&str,id:&str)->(Loader,Option<String>){let(loader,version)=owned::parse_cf_loader(mc,id);(host_loader(loader),version)}
pub async fn import_zip(state:&AppState,id:&str,bytes:Vec<u8>,fallback:Fallback)->AppResult<(PackInfo,&'static str)>{
 let fallback=owned::Fallback{mc_version:fallback.mc_version,loader:fallback.loader.map(public_loader),loader_version:fallback.loader_version};
 let(info,kind)=owned::import_zip(&Context(state),id,bytes,fallback).await.map_err(host_error)?;Ok((into_host(info),kind))
}
pub async fn apply(state:&AppState,id:&str,info:&PackInfo,kind:&str,label:&str,source_ref:serde_json::Value)->AppResult<()>{
 let info=owned::PackInfo{mc_version:info.mc_version.clone(),loader:public_loader(info.loader),loader_version:info.loader_version.clone(),name:info.name.clone(),version:info.version.clone(),files:info.files.clone()};
 owned::apply(&Context(state),id,&info,kind,label,source_ref).await.map_err(host_error)
}
pub async fn gc_files(state:&AppState,id:&str)->AppResult<()>{owned::gc_files(&Context(state),id).await.map_err(host_error)}
