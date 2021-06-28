//a stack of environments, to be used like a version tree

use super::{primitives, primitives::Primitive, values::Value};
use calyx::ir;
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::convert::TryInto;
use std::ops::DerefMut;
use std::rc::Rc;

//use push front and pop front and iterator is in right order then

struct SmoosherWr<'a, K: Eq + std::hash::Hash + Clone, V: Clone> {
    wr: &'a mut HashMap<K, V>,
}

impl<'a, K: Eq + std::hash::Hash + Clone, V: Clone> SmoosherWr<'a, K, V> {
    ///new will take in the top level hashmap from the Smoosher and form the
    ///mutable writer, lifetime annotations needed due to K and V
    fn new(hm: &'a mut HashMap<K, V>) -> SmoosherWr<'a, K, V> {
        SmoosherWr { wr: hm }
    }

    //set/write + potentially get but would be harder to use
    ///set takes in a key value pair and sets the binding of k to v in the
    ///hashmap.
    fn set(&mut self, k: K, v: V) {
        self.wr.insert(k, v);
    }
}

//get
struct SmoosherRd<'a, K: Eq + std::hash::Hash + Clone, V: Clone> {
    rd: Vec<&'a HashMap<K, V>>,
}

impl<'a, K: Eq + std::hash::Hash + Clone, V: Clone> SmoosherRd<'a, K, V> {
    ///new will recieve a vector of pointers to the hashmaps within the smoosher
    fn new(rd: Vec<&'a HashMap<K, V>>) -> Self {
        Self { rd }
    }

    fn get(&self, k: &K) -> Option<&V> {
        for hm in &self.rd {
            match hm.get(k) {
                Some(v) => return Some(v),
                None => (),
            }
        }
        None
    }
}

//two imm access for handles + smoosh (since on stack layer) + fork + push_new
struct Smoosher<K: Eq + std::hash::Hash + Clone, V: Clone> {
    ds: VecDeque<Rc<RefCell<HashMap<K, V>>>>,
    //the above is so we can keep track of scope
    //the below is to make getting easy. not sure
    //if this is too clunky
    hm: HashMap<K, V>,
}

//methods we will implement
// new, get, set, clone, top, bottom, smoosh, diff
impl<K: Eq + std::hash::Hash + Clone, V: Clone> Smoosher<K, V> {
    fn read_access(&self) -> SmoosherRd<K, V> {
        let mut coll = Vec::new();
        let mut cl: &mut Vec<&HashMap<K, V>> = Vec::borrow_mut(&mut coll);
        for hm in self.ds.range(1..) {
            cl.push(&hm.borrow());
        }
        SmoosherRd::new(cl)
    }

    fn write_access(&self) -> SmoosherWr<K, V> {
        match self.ds.front() {
            Some(v) => SmoosherWr::new(v.borrow().borrow_mut()),
            None => panic!(),
        }
    }

    fn push_new(&self) {
        self.ds.push_front(Rc::new(RefCell::new(HashMap::new())));
    }

    ///Creates a new smoosher where the top of the previous one becomes the
    ///first element of the new smoosher
    fn fork(&self) -> Smoosher<K, V> {
        let start = *self.ds.front().unwrap();
        let mut dq = VecDeque::new();
        dq.push_back(start);
        Smoosher {
            ds: dq,
            hm: HashMap::new(),
        }
    }

    fn new(k: K, v: V) -> Smoosher<K, V> {
        let hm: HashMap<K, V> = HashMap::new();
        let rc_rc_hm: Rc<RefCell<HashMap<K, V>>> = Rc::new(RefCell::new(hm));
        let mut ds: VecDeque<Rc<RefCell<HashMap<K, V>>>> = VecDeque::new();
        ds.push_back(rc_rc_hm);
        //now create a new hashmap. Invariant: This HM returns all the same
        //bindings as the HM produced by fully smooshing this Smoosher.
        let hm: HashMap<K, V> = HashMap::new();
        Smoosher { ds, hm }
    }

    //two notes:
    //make wrapper struct for read-only environment  (HashMap)
    //perhaps make internal DS vector to push all the borrows onto so they don't
    //get dropped...?
    //write_handle and read_handle internal DS so we can keep the ref alive
    //and return it

    ///get(k) returns an Option containing the most recent binding of k. As in, returns the value associated
    ///with k from the topmost HashMap that contains some key-value pair (k, v). If no HashMap exists with
    ///a key-value pair (k, v), returns None.
    fn get(&self, k: &K) -> Option<&V> {
        self.hm.get(k)
    }

    ///forgot why we put this down
    fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        self.hm.get_mut(k)
    }

    ///set(k, v) mutates the current Smoosher, inserting the key-value pair (k, v) to the topmost HashMap of
    ///the Smoosher. Overwrites the existing (k, v') pair if one exists in the topmost HashMap at the time
    ///of the set(k, v) call.
    // fn set(&mut self, k: K, v: V) {
    //     //note vecdeque can never be empty b/c initialized w/ a new hashmap
    //     if let Some(front) = self.ds.front() {
    //         let front_ref = &mut front.borrow_mut();
    //         front_ref.insert(k, v);
    //     }
    //     //should also mutate the other HM
    //     self.hm.insert(k, v);
    // }

    //note: if we change everything here to deal with Rc<RefCell...>, then clone
    //is simple we just new_scope and fork

    ///Returns a copy of the stk_env with a clean HashMap ontop (at front of internal VecDeque)

    ///Add a clean HashMap ontop of internal VecDeque
    fn new_scope(&mut self) {
        todo!()
    }

    ///Returns a RRC of the frontmost HashMap
    fn top(&self) -> &Rc<RefCell<HashMap<K, V>>> {
        self.ds.get(0).unwrap()
    }

    /// updates [bottom_i] to reflect all bindings contained in the HashMaps of indecies
    /// [bottom_i, top_i], with the higher-indecied HashMaps given precedence to
    /// their bindings, and then removes all HashMaps with index greater than [bottom_i],  
    /// note: vertical pushing down
    fn smoosh(&mut self, top_i: u64, bottom_i: u64) -> () {
        todo!()
    }

    ///merge: note: lateral (collects all forks that are parallel and merge them)
    fn merge(&mut self, other: &mut Self) -> Self {
        todo!()
    }

    fn num_scopes(&self) -> u64 {
        self.ds.len() as u64
    }

    fn num_bindings(&self) -> u64 {
        self.hm.len() as u64
    }

    ///Returns a set of all variables bound in any HashMap in the range
    ///[top_i, bottom_i). [top_i] and [bottom_i] represent distance from the top of the stack,
    /// 0 being the topmost HashMap.
    /// If [top_i] < 0 returns a set of all variables bound in any HashMap in the range [0, bottom_i]
    /// If [bottom_i] > length of stack of HashMaps, returns a set of all variables bound in any HashMap in the
    /// range [top_i, len).
    fn list_bound_vars(&self, top_i: u64, bottom_i: u64) -> HashSet<&K> {
        //note: 0 is frontmost, so i guess the terms top_i and bottom_i are
        //misleading?
        let bottom_i = if bottom_i > self.ds.len().try_into().unwrap() {
            self.ds.len().try_into().unwrap()
        } else {
            top_i
        };
        let mut hs = HashSet::new();
        let top_i = if top_i < 0 { 0 } else { top_i };
        for i in top_i..bottom_i {
            let hm = self.ds.get(i.try_into().unwrap()).unwrap(); //how to unwrap RcRefCell?
            let hm = &hm.borrow();
            for key in hm.keys() {
                hs.insert(key);
            }
        }
        hs //can't pull out references have to clone
    }

    ///in order to set unmodified values to zero
    ///
    fn diff(&self, top_i: u64, bottom_i: u64) -> Vec<(K, V)> {
        todo!()
    }
}
