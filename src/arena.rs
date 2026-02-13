use crate::typing::{ArgsRef, Record, Type, TypeRef};
use crate::{Attrs, ExprKey, ExprPtr, ExprRef, Field, RecRef, StrRef, Value, VecRef};
use rustc_hash::{FxBuildHasher, FxHashMap};
use serde::Serialize;
use std::collections::hash_map::Entry;
use std::hash::BuildHasher;
use unicase::Ascii;

/// An arena-based allocator for interning strings.
///
/// Deduplicates strings by hash and returns lightweight [`StrRef`] handles for O(1) lookups.
#[derive(Default, Serialize)]
pub struct StringArena {
    #[serde(skip_serializing)]
    hasher: FxBuildHasher,

    cache: FxHashMap<u64, StrRef>,
    slots: Vec<String>,
}

impl StringArena {
    /// Interns a string and returns its [`StrRef`]. Returns the existing reference if already interned.
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

    pub fn alloc_no_case(&mut self, value: &str) -> StrRef {
        let hash = Ascii::new(value);
        match self.cache.entry(self.hasher.hash_one(hash)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let key = StrRef(self.slots.len());
                entry.insert(key);
                self.slots.push(value.to_owned());

                key
            }
        }
    }

    /// Retrieves the string associated with the given [`StrRef`].
    pub fn get(&self, key: StrRef) -> &str {
        &self.slots[key.0]
    }

    /// Compares two interned strings for case-insensitive ASCII equality.
    pub fn eq_ignore_ascii_case(&self, ka: StrRef, kb: StrRef) -> bool {
        self.get(ka).eq_ignore_ascii_case(self.get(kb))
    }
}

/// An expression node stored in the [`ExprArena`].
///
/// Combines the expression's metadata ([`Attrs`]) with its actual content ([`Value`]).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Expr {
    /// Metadata including source position.
    pub attrs: Attrs,
    /// The kind and content of this expression.
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

    /// Allocates a vector of expression references and returns a [`VecRef`] handle.
    pub fn alloc_vec(&mut self, values: Vec<ExprRef>) -> VecRef {
        let key = VecRef(self.vecs.len());
        self.vecs.push(values);

        key
    }

    /// Allocates a vector of record fields and returns a [`RecRef`] handle.
    pub fn alloc_rec(&mut self, values: Vec<Field>) -> RecRef {
        let key = RecRef(self.recs.len());
        self.recs.push(values);

        key
    }

    /// Returns the slice of expression references for the given [`VecRef`].
    pub fn vec(&self, ptr: VecRef) -> &[ExprRef] {
        &self.vecs[ptr.0]
    }

    /// Returns the expression reference at index `idx` within the given [`VecRef`].
    pub fn vec_get(&self, ptr: VecRef, idx: usize) -> ExprRef {
        self.vecs[ptr.0][idx]
    }

    /// Returns an iterator over valid indices for the given [`VecRef`].
    pub fn vec_idxes(&self, ptr: VecRef) -> impl Iterator<Item = usize> + use<> {
        0..self.vec(ptr).len()
    }

    /// Returns the vector of fields for the given [`RecRef`].
    pub fn rec(&self, ptr: RecRef) -> &Vec<Field> {
        &self.recs[ptr.0]
    }

    /// Returns the field at index `idx` within the given [`RecRef`].
    pub fn rec_get(&self, ptr: RecRef, idx: usize) -> Field {
        self.recs[ptr.0][idx]
    }

    /// Returns an iterator over valid indices for the given [`RecRef`].
    pub fn rec_idxes(&self, ptr: RecRef) -> impl Iterator<Item = usize> + use<> {
        0..self.rec(ptr).len()
    }
}

/// An arena-based allocator for type information.
///
/// Stores and deduplicates types, record definitions, and function argument lists.
/// Supports freezing to mark a baseline and freeing types allocated after the baseline.
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
    /// Marks the current allocation state as the baseline.
    ///
    /// Subsequent calls to [`free_space`](TypeArena::free_space) will deallocate
    /// only types and records allocated after this point.
    pub fn freeze(&mut self) {
        self.rec_offset = self.records.len();
        self.type_offset = self.types.len();
    }

    /// Frees types and records allocated after the last [`freeze`](TypeArena::freeze) call.
    pub fn free_space(&mut self) {
        for tpe in self.types.drain(self.type_offset..) {
            self.dedup_types.remove(&tpe);
        }

        for _ in self.records.drain(self.rec_offset..) {}
    }

    /// Registers a type and returns a deduplicated [`TypeRef`]. Returns the existing reference if already registered.
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

    /// Allocates a fresh copy of a type. For records, this clones the record definition.
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

    /// Creates an array type containing elements of the given type.
    pub fn alloc_array_of(&mut self, tpe: Type) -> Type {
        Type::Array(self.register_type(tpe))
    }

    /// Allocates a new record type from a map of field names to types.
    pub fn alloc_record(&mut self, record: FxHashMap<StrRef, Type>) -> Record {
        let key = Record(self.records.len());
        self.records.push(record);
        key
    }

    /// Allocates a deduplicated list of function argument types and returns an [`ArgsRef`].
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

    /// Retrieves the type for the given [`TypeRef`].
    pub fn get_type(&self, key: TypeRef) -> Type {
        self.types[key.0]
    }

    /// Returns the field map for the given record.
    pub fn get_record(&self, key: Record) -> &FxHashMap<StrRef, Type> {
        &self.records[key.0]
    }

    /// Returns the argument type slice for the given [`ArgsRef`].
    pub fn get_args(&self, key: ArgsRef) -> &[Type] {
        self.args[key.0].as_slice()
    }

    /// Returns a mutable reference to the argument type slice for the given [`ArgsRef`].
    pub fn get_args_mut(&mut self, key: ArgsRef) -> &mut [Type] {
        self.args[key.0].as_mut_slice()
    }

    /// Returns an iterator over valid indices for the given [`ArgsRef`].
    pub fn args_idxes(&self, key: ArgsRef) -> impl Iterator<Item = usize> + use<> {
        0..self.get_args(key).len()
    }

    /// Returns the argument type at index `idx` for the given [`ArgsRef`].
    pub fn args_get(&self, key: ArgsRef, idx: usize) -> Type {
        self.get_args(key)[idx]
    }

    /// Returns the type of a field in the given record, or `None` if the field doesn't exist.
    pub fn record_get(&self, record: Record, field: StrRef) -> Option<Type> {
        self.records[record.0].get(&field).copied()
    }

    /// Iterates over all (field name, type) pairs in the given record.
    pub fn record_iter(&self, record: Record) -> impl Iterator<Item = (StrRef, Type)> {
        self.records[record.0].iter().map(|(k, v)| (*k, *v))
    }

    /// Iterates over all field names in the given record.
    pub fn record_keys(&self, record: Record) -> impl Iterator<Item = StrRef> {
        self.records[record.0].keys().copied()
    }

    /// Checks whether two records have the exact same set of field names.
    pub fn records_have_same_keys(&self, rec_a: Record, rec_b: Record) -> bool {
        let rec_a = self.get_record(rec_a);
        let rec_b = self.get_record(rec_b);

        if rec_a.is_empty() && rec_b.is_empty() {
            return true;
        }

        if rec_a.len() != rec_b.len() {
            return false;
        }

        for bk in rec_b.keys() {
            if !rec_a.contains_key(bk) {
                return false;
            }
        }

        true
    }

    /// Creates an empty record type.
    pub fn instantiate_record(&mut self) -> Record {
        self.alloc_record(FxHashMap::default())
    }

    /// Returns `true` if the given field exists in the record.
    pub fn record_field_exists(&self, record: Record, field: StrRef) -> bool {
        self.records[record.0].contains_key(&field)
    }

    /// Returns the hash map entry for a field in the given record, for in-place manipulation.
    pub fn record_entry(&mut self, record: Record, key: StrRef) -> Entry<'_, StrRef, Type> {
        self.records[record.0].entry(key)
    }

    /// Sets the type of a field in the given record, inserting or updating as needed.
    pub fn record_set(&mut self, record: Record, field: StrRef, value: Type) {
        self.records[record.0].insert(field, value);
    }

    /// Returns the number of fields in the given record.
    pub fn record_len(&self, record: Record) -> usize {
        self.records[record.0].len()
    }

    /// Returns `true` if the given record has no fields.
    pub fn record_is_empty(&self, record: Record) -> bool {
        self.records[record.0].is_empty()
    }
}

/// Top-level arena that holds all memory pools for expressions, strings, and types.
#[derive(Default, Serialize)]
pub struct Arena {
    pub(crate) exprs: ExprArena,
    pub(crate) strings: StringArena,
    pub(crate) types: TypeArena,
}

impl Arena {
    /// Freezes the type arena to mark the current state as baseline.
    pub fn freeze(&mut self) {
        self.types.freeze();
    }

    /// Frees types allocated after the last freeze, reclaiming memory for reuse.
    pub fn free_space(&mut self) {
        self.types.free_space();
    }
}
