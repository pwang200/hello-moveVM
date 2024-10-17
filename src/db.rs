use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::fmt::Debug;
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChangeSet, ChangeSet, Op},
    identifier::Identifier,
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
    resolver::{resource_size, ModuleResolver,
               ResourceResolver},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use bytes::Bytes;
use move_binary_format::errors::{PartialVMError, PartialVMResult};

#[derive(Debug, Clone)]
struct AccountStorage {
    resources: HashMap<StructTag, Bytes>,
    modules: HashMap<Identifier, Bytes>,
}

impl AccountStorage {
    fn new() -> Self {
        Self {
            modules: HashMap::new(),
            resources: HashMap::new(),
        }
    }

    fn apply(&mut self, changes: AccountChangeSet) -> PartialVMResult<()> {
        let (modules, resources) = changes.into_inner();
        apply_changes(&mut self.modules, modules)?;
        apply_changes(&mut self.resources, resources)?;
        Ok(())
    }
}

fn apply_changes<K, V>(
    map: &mut HashMap<K, V>,
    changes: impl IntoIterator<Item=(K, Op<V>)>,
) -> PartialVMResult<()>
    where
        K: Ord + Debug + std::hash::Hash,
{
    use Op::*;

    for (k, op) in changes.into_iter() {
        match (map.entry(k), op) {
            (Occupied(entry), New(_)) => {
                return Err(
                    PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(format!(
                        "Failed to apply changes -- key {:?} already exists",
                        entry.key()
                    )),
                );
            }
            (Occupied(entry), Delete) => {
                entry.remove();
            }
            (Occupied(entry), Modify(val)) => {
                *entry.into_mut() = val;
            }
            (Vacant(entry), New(val)) => {
                entry.insert(val);
            }
            (Vacant(entry), Delete | Modify(_)) => {
                return Err(
                    PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(format!(
                        "Failed to apply changes -- key {:?} does not exist",
                        entry.key()
                    )),
                );
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct MockStorage {
    accounts: HashMap<AccountAddress, AccountStorage>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    pub fn apply_extended(
        &mut self,
        changeset: ChangeSet,
    ) -> PartialVMResult<()> {
        for (addr, account_changeset) in changeset.into_inner() {
            match self.accounts.entry(addr) {
                Occupied(entry) => {
                    entry.into_mut().apply(account_changeset)?;
                }
                Vacant(entry) => {
                    let mut account_storage = AccountStorage::new();
                    account_storage.apply(account_changeset)?;
                    entry.insert(account_storage);
                }
            }
        }

        Ok(())
    }

    pub fn apply(&mut self, changeset: ChangeSet) -> PartialVMResult<()> {
        self.apply_extended(changeset)
    }

    pub fn publish_or_overwrite_module(&mut self, module_id: ModuleId, blob: Vec<u8>) {
        let account = self.accounts.entry(module_id.address).or_insert(AccountStorage::new());
        account.modules.insert(module_id.name().to_owned(), blob.into());
    }

    pub fn publish_or_overwrite_resource(
        &mut self,
        addr: AccountAddress,
        struct_tag: StructTag,
        blob: Vec<u8>,
    ) {
        let account =  self.accounts.entry(addr).or_insert(AccountStorage::new());
        account.resources.insert(struct_tag, blob.into());
    }
}

impl ModuleResolver for MockStorage {
    type Error = PartialVMError;

    fn get_module_metadata(&self, _module_id: &ModuleId) -> Vec<Metadata> {
        vec![]
    }

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Bytes>, Self::Error> {
        if let Some(account_storage) = self.accounts.get(module_id.address()) {
            return Ok(account_storage.modules.get(module_id.name()).cloned());
        }
        Ok(None)
    }
}

impl ResourceResolver for MockStorage {
    type Error = PartialVMError;

    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
        _metadata: &[Metadata],
        _maybe_layout: Option<&MoveTypeLayout>,
    ) -> Result<(Option<Bytes>, usize), Self::Error> {
        if let Some(account_storage) = self.accounts.get(address) {
            let buf = account_storage.resources.get(tag).cloned();
            let buf_size = resource_size(&buf);
            return Ok((buf, buf_size));
        }
        Ok((None, 0))
    }
}