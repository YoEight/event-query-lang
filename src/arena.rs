use crate::typing::{ArgsRef, Record, Type, TypeRef};
use crate::{Attrs, ExprKey, ExprPtr, ExprRef, Field, RecRef, StrRef, Value, VecRef};
use rustc_hash::{FxBuildHasher, FxHashMap};
use serde::Serialize;
use std::collections::hash_map::Entry;
use std::hash::BuildHasher;

#[derive(Default, Serialize)]
pub struct StringArena {
    #[serde(skip_serializing)]
    hasher: FxBuildHasher,

    cache: FxHashMap<u64, StrRef>,
    slots: Vec<String>,
}

impl StringArena {
    pub fn alloc(&mut self, value: &str) -> StrRef {
        match self.cache.entry(self.hasher.hash_one(value)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let key = StrRef(self.slots.len());
                entry.insert(key);
                self.slots.push(value.to_owned());

                key
            }
        }
    }

    pub fn get(&self, key: StrRef) -> &str {
        &self.slots[key.0]
    }

    pub fn eq_ignore_ascii_case(&self, ka: StrRef, kb: StrRef) -> bool {
        self.get(ka).eq_ignore_ascii_case(self.get(kb))
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Expr {
    pub attrs: Attrs,
    pub value: Value,
}

/// An arena-based allocator for EventQL expressions.
///
/// The `ExprArena` provides a memory-efficient way to store and manage AST nodes
/// by using a flat vector and returning lightweight [`ExprRef`] handles.
#[derive(Default, Serialize)]
pub struct ExprArena {
    #[serde(skip_serializing)]
    hasher: FxBuildHasher,
    exprs: Vec<Expr>,
    vecs: Vec<Vec<ExprRef>>,
    recs: Vec<Vec<Field>>,
}

impl ExprArena {
    /// Allocates a new expression in the arena.
    ///
    /// This method takes an expression's attributes and value, hashes the value
    /// to create a stable [`ExprKey`], and stores it in the arena. It returns
    /// an [`ExprRef`] which can be used to retrieve the expression later.
    pub fn alloc(&mut self, attrs: Attrs, value: Value) -> ExprRef {
        let key = ExprKey(self.hasher.hash_one(value));

        let ptr = ExprPtr(self.exprs.len());
        self.exprs.push(Expr { attrs, value });

        ExprRef { key, ptr }
    }

    /// Retrieves a node from the arena using an [`ExprRef`].
    ///
    /// # Panics
    ///
    /// Panics if the [`ExprRef`] contains an invalid pointer that is out of bounds
    /// of the arena's internal storage.
    pub fn get(&self, node_ref: ExprRef) -> Expr {
        self.exprs[node_ref.ptr.0]
    }

    pub fn alloc_vec(&mut self, values: Vec<ExprRef>) -> VecRef {
        let key = VecRef(self.vecs.len());
        self.vecs.push(values);

        key
    }

    pub fn alloc_rec(&mut self, values: Vec<Field>) -> RecRef {
        let key = RecRef(self.recs.len());
        self.recs.push(values);

        key
    }

    pub fn vec(&self, ptr: VecRef) -> &[ExprRef] {
        &self.vecs[ptr.0]
    }

    pub fn vec_get(&self, ptr: VecRef, idx: usize) -> ExprRef {
        self.vecs[ptr.0][idx]
    }

    pub fn vec_idxes(&self, ptr: VecRef) -> impl Iterator<Item = usize> + use<> {
        0..self.vec(ptr).len()
    }

    pub fn rec(&self, ptr: RecRef) -> &Vec<Field> {
        &self.recs[ptr.0]
    }

    pub fn rec_get(&self, ptr: RecRef, idx: usize) -> Field {
        self.recs[ptr.0][idx]
    }

    pub fn rec_idxes(&self, ptr: RecRef) -> impl Iterator<Item = usize> + use<> {
        0..self.rec(ptr).len()
    }
}

#[derive(Default, Serialize)]
pub struct TypeArena {
    #[serde(skip_serializing)]
    args_hasher: FxBuildHasher,

    type_offset: usize,
    rec_offset: usize,

    dedup_types: FxHashMap<Type, TypeRef>,
    dedup_args: FxHashMap<u64, ArgsRef>,
    types: Vec<Type>,
    pub(crate) records: Vec<FxHashMap<StrRef, Type>>,
    pub(crate) args: Vec<Vec<Type>>,
}

impl TypeArena {
    pub fn freeze(&mut self) {
        self.rec_offset = self.records.len();
        self.type_offset = self.types.len();
    }

    pub fn free_space(&mut self) {
        for tpe in self.types.drain(self.type_offset..) {
            self.dedup_types.remove(&tpe);
        }

        for _ in self.records.drain(self.rec_offset..) {}
    }

    pub fn register_type(&mut self, tpe: Type) -> TypeRef {
        match self.dedup_types.entry(tpe) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let key = TypeRef(self.types.len());
                self.types.push(tpe);
                entry.insert(key);

                key
            }
        }
    }

    pub fn alloc_type(&mut self, tpe: Type) -> Type {
        if let Type::Record(rec) = tpe {
            let key = Record(self.records.len());
            // TODO: technically, a deep-clone is needed here, where properties that point to
            // records should also be allocated as well.
            self.records.push(self.records[rec.0].clone());

            return Type::Record(key);
        }

        tpe
    }

    pub fn alloc_array_of(&mut self, tpe: Type) -> Type {
        Type::Array(self.register_type(tpe))
    }

    pub fn alloc_record(&mut self, record: FxHashMap<StrRef, Type>) -> Record {
        let key = Record(self.records.len());
        self.records.push(record);
        key
    }

    pub fn alloc_args(&mut self, args: &[Type]) -> ArgsRef {
        let hash = self.args_hasher.hash_one(args);

        match self.dedup_args.entry(hash) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let key = ArgsRef(self.args.len());
                entry.insert(key);
                self.args.push(args.to_vec());

                key
            }
        }
    }

    pub fn get_type(&self, key: &TypeRef) -> Type {
        self.types[key.0]
    }

    pub fn get_record(&self, key: &Record) -> &FxHashMap<StrRef, Type> {
        &self.records[key.0]
    }

    pub fn get_args(&self, key: &ArgsRef) -> &[Type] {
        self.args[key.0].as_slice()
    }

    pub fn get_args_mut(&mut self, key: &ArgsRef) -> &mut [Type] {
        self.args[key.0].as_mut_slice()
    }

    pub fn args_idxes(&self, key: ArgsRef) -> impl Iterator<Item = usize> + use<> {
        0..self.get_args(&key).len()
    }

    pub fn args_get(&self, key: ArgsRef, idx: usize) -> Type {
        self.get_args(&key)[idx]
    }

    pub fn record_get(&self, record: Record, field: StrRef) -> Option<Type> {
        self.records[record.0].get(&field).copied()
    }

    pub fn record_iter(&self, record: Record) -> impl Iterator<Item = (StrRef, Type)> {
        self.records[record.0].iter().map(|(k, v)| (*k, *v))
    }

    pub fn record_keys(&self, record: Record) -> impl Iterator<Item = StrRef> {
        self.records[record.0].keys().copied()
    }

    pub fn records_have_same_keys(&self, reca: Record, recb: Record) -> bool {
        let reca = self.get_record(&reca);
        let recb = self.get_record(&recb);

        if reca.is_empty() && recb.is_empty() {
            return true;
        }

        if reca.len() != recb.len() {
            return false;
        }

        for bk in recb.keys() {
            if !reca.contains_key(bk) {
                return false;
            }
        }

        true
    }

    pub fn instantiate_record(&mut self) -> Record {
        self.alloc_record(FxHashMap::default())
    }

    pub fn record_field_exists(&self, record: Record, field: StrRef) -> bool {
        self.records[record.0].contains_key(&field)
    }

    pub fn record_entry(&mut self, record: Record, key: StrRef) -> Entry<'_, StrRef, Type> {
        self.records[record.0].entry(key)
    }

    pub fn record_set(&mut self, record: Record, field: StrRef, value: Type) {
        self.records[record.0].insert(field, value);
    }

    pub fn record_len(&self, record: Record) -> usize {
        self.records[record.0].len()
    }

    pub fn record_is_empty(&self, record: Record) -> bool {
        self.records[record.0].is_empty()
    }
}

#[derive(Default, Serialize)]
pub struct Arena {
    pub(crate) exprs: ExprArena,
    pub(crate) strings: StringArena,
    pub(crate) types: TypeArena,
}

impl Arena {
    pub fn freeze(&mut self) {
        self.types.freeze();
    }

    pub fn free_space(&mut self) {
        self.types.free_space();
    }

    pub fn alloc_str(&mut self, value: &str) -> StrRef {
        self.strings.alloc(value)
    }

    pub fn eq_ignore_ascii_case(&self, ka: StrRef, kb: StrRef) -> bool {
        self.strings
            .get(ka)
            .eq_ignore_ascii_case(self.strings.get(kb))
    }

    pub fn get_str(&self, key: &StrRef) -> &str {
        self.strings.get(*key)
    }

    pub fn exprs(&self) -> &ExprArena {
        &self.exprs
    }

    pub fn exprs_mut(&mut self) -> &mut ExprArena {
        &mut self.exprs
    }

    pub fn types(&self) -> &TypeArena {
        &self.types
    }

    pub fn types_mut(&mut self) -> &mut TypeArena {
        &mut self.types
    }
}
