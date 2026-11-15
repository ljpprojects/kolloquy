use chrono::Utc;
use kolloquy_consts::PASS2_THYME_SIZE;
use sqlx::{FromRow, SqlSafeStr};
use std::sync::Arc;
use std::{
    mem,
    ops::{BitAnd, BitOr, Deref},
    path::Path,
};
use zeroize::Zeroizing;

#[cfg(feature = "postgres-db")]
fn q() {
    sqlx::query_as().fetch_one();
}

// There used to be a DatabaseComponent before I realised it was redundant, as it was just a wrapper
// sqlx

pub trait StoragePathInfo {
    // When the data at the path was created
    fn created_date(&self) -> chrono::DateTime<Utc>;

    // When the data at the path was last modified
    fn modified_date(&self) -> chrono::DateTime<Utc>;

    // When the data at the path was last accesssed (getting info does not count)
    fn accessed_date(&self) -> chrono::DateTime<Utc>;

    // Whether or not the data at the path is write-protected. Deletion is NOT
    // covered under this.
    fn wr_prot(&self) -> bool;

    // Whether or not the data at the path is delete-protected.
    fn del_prot(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Protection {
    None = 0,
    WriteProt = 1,
    DeleteProt = 1 << 1,
}

impl Deref for Protection {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        unsafe { mem::transmute(self) }
    }
}

impl BitOr for Protection {
    type Output = Protection;

    fn bitor(self, rhs: Self) -> Self::Output {
        unsafe { mem::transmute(&*self | &*rhs) }
    }
}

impl BitAnd for Protection {
    type Output = Protection;

    fn bitand(self, rhs: Self) -> Self::Output {
        unsafe { mem::transmute(&*self & &*rhs) }
    }
}

/// A trait for storage components. This is used to store things like
/// attachments, user profile pictures, and encrypted message contents.
pub trait StorageComponent {
    type Error: std::error::Error;
    type Info: StoragePathInfo;

    fn read_at_path(
        &self,
        path: &Path,
        out: &mut Vec<u8>,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    fn write_at_path(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> impl Future<Output = Result<(), Self::Error>>;

    fn create_at_path(
        &self,
        path: &Path,
        prot: &Protection,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    fn delete_at_path(&self, path: &Path) -> impl Future<Output = Result<(), Self::Error>>;

    fn info_at_path(&self, path: &Path) -> impl Future<Output = Result<Self::Info, Self::Error>>;

    fn get_xattr_at_path(
        &self,
        path: &Path,
        attr_name: &str,
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>>;

    fn set_xattr_at_path(
        &self,
        path: &Path,
        attr_name: &str,
        attr_val: &[u8],
    ) -> impl Future<Output = Result<(), Self::Error>>;
}

/// A trait for server state. This is used to hold the stage 2 secret,
/// referred to as the "thyme". You can extend the state as needed.
pub trait StateComponent {
    fn pass2_thyme(&self) -> Arc<Zeroizing<[u8; PASS2_THYME_SIZE]>>;
}

#[cfg(feature = "default-components")]
mod _default_components {
    use crate::component::{Protection, StateComponent, StorageComponent, StoragePathInfo};
    use chrono::{DateTime, Utc};
    use kolloquy_consts::PASS2_THYME_SIZE;
    use std::fmt::Display;
    use std::io::Write;
    use std::ops::BitOrAssign;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use std::sync::Arc;
    use tokio::io::AsyncWriteExt;
    use xattr::FileExt;
    use zeroize::Zeroizing;

    pub struct ServerState {
        thyme: Arc<Zeroizing<[u8; PASS2_THYME_SIZE]>>,
    }

    impl StateComponent for ServerState {
        fn pass2_thyme(&self) -> Arc<Zeroizing<[u8; PASS2_THYME_SIZE]>> {
            self.thyme.clone()
        }
    }

    #[derive(Debug)]
    pub enum DirStorageError {
        NoSuchPath,
        NoSuchAttribute,
        WriteProtectedFile,
        DeleteProtectedFile,
        IoError(std::io::Error),
        JoinError(tokio::task::JoinError),
    }

    impl Display for DirStorageError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                DirStorageError::NoSuchPath => write!(f, "no such path"),
                DirStorageError::NoSuchAttribute => write!(f, "no such attribute"),
                DirStorageError::WriteProtectedFile => write!(f, "write protected file"),
                DirStorageError::DeleteProtectedFile => write!(f, "delete protected file"),
                DirStorageError::IoError(e) => write!(f, "io error: {e}"),
                DirStorageError::JoinError(e) => write!(f, "join error: {e}"),
            }
        }
    }

    impl std::error::Error for DirStorageError {}

    pub enum ProtPolicy {
        Match,
        Transform(Box<dyn Fn(&Protection) -> Protection>),
        Restrict,
        Allow,
    }

    fn run_prot_policy(p: &Protection, pol: &ProtPolicy) -> Protection {
        match pol {
            ProtPolicy::Match => *p,
            ProtPolicy::Transform(f) => f(p),
            ProtPolicy::Restrict => Protection::DeleteProt | Protection::WriteProt,
            ProtPolicy::Allow => Protection::None,
        }
    }

    fn prot_to_perm(p: &Protection) -> u32 {
        let mut perm = 2 & 4; // The files are never executable by default
        if *p & Protection::WriteProt {
            perm &= !2; // Get rid of write permissions
        };

        // Delete permissions aren't managed by the OS but by the component (via
        // extended attributes on the file)

        perm
    }

    // Default storage component: store data in the files of a directory
    pub struct DirectoryStorage {
        dir_path: Box<Path>,
        group_prot_policy: ProtPolicy,
        everyone_prot_policy: ProtPolicy,
    }

    pub struct DirStoragePathInfo {
        last_access: DateTime<Utc>,
        modified: DateTime<Utc>,
        created: DateTime<Utc>,
        prots: Protection,
    }

    impl StoragePathInfo for DirStoragePathInfo {
        fn created_date(&self) -> DateTime<Utc> {
            self.created
        }

        fn modified_date(&self) -> DateTime<Utc> {
            self.modified
        }

        fn accessed_date(&self) -> DateTime<Utc> {
            self.last_access
        }

        fn wr_prot(&self) -> bool {
            self.prots & Protection::WriteProt == Protection::WriteProt
        }

        fn del_prot(&self) -> bool {
            self.prots & Protection::DeleteProt == Protection::DeleteProt
        }
    }

    impl BitOrAssign for Protection {
        fn bitor_assign(&mut self, rhs: Self) {
            *self = *self | rhs
        }
    }

    impl StorageComponent for DirectoryStorage {
        type Error = DirStorageError;
        type Info = DirStoragePathInfo;

        async fn read_at_path(&self, path: &Path, out: &mut Vec<u8>) -> Result<(), Self::Error> {
            let contents = tokio::fs::read(self.dir_path.join(path))
                .await
                .map_err(|e| DirStorageError::IoError(e))?;

            out.extend_from_slice(&contents);

            Ok(())
        }

        async fn write_at_path(&self, path: &Path, bytes: &[u8]) -> Result<(), Self::Error> {
            let path = self.dir_path.join(path);

            let f = tokio::fs::File::open(path)
                .await
                .map_err(|e| DirStorageError::IoError(e))?
                .into_std()
                .await;

            let write_prot_en = tokio::task::spawn_blocking(|| f.get_xattr("user.klqy_write_prot"))
                .await
                .map_err(|e| DirStorageError::JoinError(e))?
                .map_err(|e| DirStorageError::IoError(e))?
                .map(|v| v[0])
                .unwrap_or(0);

            if write_prot_en == b'Y' {
                return Err(DirStorageError::DeleteProtectedFile);
            }

            tokio::fs::File::from_std(f)
                .write_all(&bytes)
                .await
                .map_err(|e| DirStorageError::IoError(e))
        }

        async fn create_at_path(&self, path: &Path, prot: &Protection) -> Result<(), Self::Error> {
            let path = self.dir_path.join(path);
            let f = tokio::fs::File::create(path)
                .await
                .map_err(|e| DirStorageError::IoError(e))?;

            let do_prot = |p, pol| prot_to_perm(&run_prot_policy(prot, pol));

            let perms: u32 = prot_to_perm(prot) << 6
                & do_prot(prot, &self.group_prot_policy) << 3
                & do_prot(prot, &self.everyone_prot_policy);

            let mut p = f
                .metadata()
                .await
                .map_err(|e| DirStorageError::IoError(e))?
                .permissions();

            p.set_mode(perms);

            f.set_permissions(p)
                .await
                .map_err(|e| DirStorageError::IoError(e))?;

            let std_file = f.into_std().await;

            // We do delete protection via an xattr (the delete protection extends only to this
            // component, not the fs)
            if *prot & Protection::DeleteProt == Protection::DeleteProt {
                tokio::task::spawn_blocking(|| std_file.set_xattr("user.klqy_delete_prot", b"Y"))
                    .await
                    .map_err(|e| DirStorageError::JoinError(e))?
                    .map_err(|e| DirStorageError::IoError(e))?;
            }

            if *prot & Protection::WriteProt == Protection::WriteProt {
                tokio::task::spawn_blocking(|| std_file.set_xattr("user.klqy_write_prot", b"Y"))
                    .await
                    .map_err(|e| DirStorageError::JoinError(e))?
                    .map_err(|e| DirStorageError::IoError(e))?;
            }

            Ok(())
        }

        async fn delete_at_path(&self, path: &Path) -> Result<(), Self::Error> {
            let path = self.dir_path.join(path);
            let f = tokio::fs::File::open(path)
                .await
                .map_err(|e| DirStorageError::IoError(e))?
                .into_std()
                .await;

            let delete_prot_en =
                tokio::task::spawn_blocking(|| f.get_xattr("user.klqy_delete_prot"))
                    .await
                    .map_err(|e| DirStorageError::JoinError(e))?
                    .map_err(|e| DirStorageError::IoError(e))?
                    .map(|v| v[0])
                    .unwrap_or(0);

            if delete_prot_en == b'Y' {
                return Err(DirStorageError::DeleteProtectedFile);
            }

            tokio::fs::remove_file(path)
        }

        async fn info_at_path(&self, path: &Path) -> Result<Self::Info, Self::Error> {
            let path = self.dir_path.join(path);
            let f = tokio::fs::File::open(path)
                .await
                .map_err(|e| DirStorageError::IoError(e))?;

            let m = f
                .metadata()
                .await
                .map_err(|e| DirStorageError::IoError(e))?;
            let f = f.into_std().await;

            let mut p = Protection::None;

            let delete_prot_en =
                tokio::task::spawn_blocking(|| f.get_xattr("user.klqy_delete_prot"))
                    .await
                    .map_err(|e| DirStorageError::JoinError(e))?
                    .map_err(|e| DirStorageError::IoError(e))?
                    .map(|v| v[0])
                    .unwrap_or(0);

            let write_prot_en = tokio::task::spawn_blocking(|| f.get_xattr("user.klqy_write_prot"))
                .await
                .map_err(|e| DirStorageError::JoinError(e))?
                .map_err(|e| DirStorageError::IoError(e))?
                .map(|v| v[0])
                .unwrap_or(0);

            if delete_prot_en == b'Y' {
                p |= Protection::DeleteProt;
            }

            if write_prot_en == b'Y' {
                p |= Protection::WriteProt;
            }

            Ok(DirStoragePathInfo {
                last_access: DateTime::from(m.accessed().map_err(|e| DirStorageError::IoError(e))?),
                created: DateTime::from(m.created().map_err(|e| DirStorageError::IoError(e))?),
                modified: DateTime::from(m.modified().map_err(|e| DirStorageError::IoError(e))?),
                prots: p,
            })
        }

        async fn get_xattr_at_path(
            &self,
            path: &Path,
            attr_name: &str,
        ) -> Result<Vec<u8>, Self::Error> {
            let path = self.dir_path.join(path);
            let f = tokio::fs::File::open(path)
                .await
                .map_err(|e| DirStorageError::IoError(e))?
                .into_std()
                .await;

            tokio::task::spawn_blocking(|| f.get_xattr(attr_name))
                .await
                .map_err(|e| DirStorageError::JoinError(e))?
                .map_err(|e| DirStorageError::IoError(e))?
                .map(|v| Ok(v))
                .map_err(|e| DirStorageError::IoError(e))
        }

        async fn set_xattr_at_path(
            &self,
            path: &Path,
            attr_name: &str,
            attr_val: &[u8],
        ) -> Result<(), Self::Error> {
            let path = self.dir_path.join(path);
            let f = tokio::fs::File::open(path)
                .await
                .map_err(|e| DirStorageError::IoError(e))?
                .into_std()
                .await;

            tokio::task::spawn_blocking(|| f.set_xattr(attr_name, attr_val))
                .await
                .map_err(|e| DirStorageError::JoinError(e))?
                .map_err(|e| DirStorageError::IoError(e))
        }
    }
}

#[cfg(feature = "default-components")]
pub use _default_components::*;

// It is cheaper to use a donor computer IF AND ONLY IF it comes with all of its
// parts I would have to otherwise buy separately (WLAN and WWAN card, display,
// RAM, SSD)
