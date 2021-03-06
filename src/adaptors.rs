//! Licensed under the Apache License, Version 2.0
//! http://www.apache.org/licenses/LICENSE-2.0 or the MIT license
//! http://opensource.org/licenses/MIT, at your
//! option. This file may not be copied, modified, or distributed
//! except according to those terms.

use std::mem;
use std::num::Int;
use std::usize;
use std::iter::{Fuse, Peekable};
use super::Itertools;

/// Alternate elements from two iterators until both
/// are run out
///
/// Iterator element type is `A` if `I: Iterator<A>`
#[derive(Clone)]
pub struct Interleave<I, J> {
    a: I,
    b: J,
    flag: bool,
}

impl<I, J> Interleave<I, J> {
    ///
    pub fn new(a: I, b: J) -> Interleave<I, J> {
        Interleave{a: a, b: b, flag: false}
    }
}

impl<A, I, J> Iterator for Interleave<I, J>
    where I: Iterator<Item=A>, J: Iterator<Item=A>
{
    type Item = A;
    #[inline]
    fn next(&mut self) -> Option<A> {
        self.flag = !self.flag;
        if self.flag {
            match self.a.next() {
                None => self.b.next(),
                r => r,
            }
        } else {
            match self.b.next() {
                None => self.a.next(),
                r => r,
            }
        }
    }
}

/// Clonable iterator adaptor to map elementwise
/// from `Iterator<A>` to `Iterator<B>`
///
/// Created with `.fn_map(..)` on an iterator
///
/// Iterator element type is `B`
#[deprecated]
pub struct FnMap<A, B, I> {
    map: fn(A) -> B,
    iter: I,
}

impl<A, B, I> FnMap<A, B, I>
{
    pub fn new(iter: I, map: fn(A) -> B) -> FnMap<A, B, I> {
        FnMap{iter: iter, map: map}
    }
}

impl<A, B, I: Iterator<Item=A>> Iterator for FnMap<A, B, I>
{
    type Item = B;
    #[inline]
    fn next(&mut self) -> Option<B> {
        self.iter.next().map(|a| (self.map)(a))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<A, B, I> DoubleEndedIterator for FnMap<A, B, I>
    where I: DoubleEndedIterator<Item=A>
{
    #[inline]
    fn next_back(&mut self) -> Option<B> {
        self.iter.next_back().map(|a| (self.map)(a))
    }
}

impl<A, B, I: Clone> Clone for FnMap<A, B, I>
{
    fn clone(&self) -> FnMap<A, B, I> {
        FnMap::new(self.iter.clone(), self.map)
    }
}

/// An iterator adaptor that allows putting back a single
/// item to the front of the iterator.
///
/// Iterator element type is `A`
#[derive(Clone)]
pub struct PutBack<A, I> {
    top: Option<A>,
    iter: I
}

impl<A, I> PutBack<A, I> {
    /// Iterator element type is `A`
    #[inline]
    pub fn new(it: I) -> PutBack<A, I> {
        PutBack{top: None, iter: it}
    }

    /// Put back a single value to the front of the iterator.
    ///
    /// If a value is already in the put back slot, it is overwritten.
    #[inline]
    pub fn put_back(&mut self, x: A) {
        self.top = Some(x)
    }
}

impl<A, I> Iterator for PutBack<A, I>
    where I: Iterator<Item=A>
{
    type Item = A;
    #[inline]
    fn next(&mut self) -> Option<A> {
        match self.top {
            None => self.iter.next(),
            ref mut some => some.take(),
        }
    }
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lo, hi) = self.iter.size_hint();
        match self.top {
            Some(_) => (lo.saturating_add(1), hi.and_then(|x| x.checked_add(1))),
            None => (lo, hi)
        }
    }
}


/// An iterator adaptor that iterates over the cartesian product of
/// the element sets of two iterators `I` and `J`.
///
/// Iterator element type is `(A, B)` if `I: Iterator<A>` and `J: Iterator<B>`
#[derive(Clone)]
pub struct Product<A, I, J> {
    a: I,
    a_cur: Option<A>,
    b: J,
    b_orig: J,
}

impl<A: Clone, B, I: Iterator<Item=A>, J: Clone + Iterator<Item=B>>
    Product<A, I, J> 
{
    /// Create a new cartesian product iterator
    ///
    /// Iterator element type is `(A, B)` if `I: Iterator<A>` and `J: Iterator<B>`
    pub fn new(i: I, j: J) -> Product<A, I, J>
    {
        let mut i = i;
        Product{a_cur: i.next(), a: i, b: j.clone(), b_orig: j}
    }
}


impl<A: Clone, I: Iterator<Item=A>, B, J: Clone + Iterator<Item=B>>
Iterator for Product<A, I, J>
{
    type Item = (A, B);
    fn next(&mut self) -> Option<(A, B)>
    {
        let elt_b = match self.b.next() {
            None => {
                self.b = self.b_orig.clone();
                match self.b.next() {
                    None => return None,
                    Some(x) => {
                        self.a_cur = self.a.next();
                        x
                    }
                }
            }
            Some(x) => x
        };
        match self.a_cur {
            None => None,
            Some(ref a) => {
                Some((a.clone(), elt_b))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>)
    {
        let (a, ah) = self.a.size_hint();
        let (b, bh) = self.b.size_hint();
        let (bo, boh) = self.b_orig.size_hint();

        // Compute a * bo + b for both lower and upper bound
        let low = a.checked_mul(bo)
                    .and_then(|x| x.checked_add(b))
                    .unwrap_or(::std::usize::MAX);
        let high = ah.and_then(|x| boh.and_then(|y| x.checked_mul(y)))
                     .and_then(|x| bh.and_then(|y| x.checked_add(y)));
        (low, high)
    }
}

/// Remove duplicates from sections of consecutive identical elements.
/// If the iterator is sorted, all elements will be unique.
///
/// Iterator element type is `A` if `I: Iterator<A>`
#[derive(Clone)]
pub struct Dedup<A, I> {
    last: Option<A>,
    iter: I,
}

impl<A, I> Dedup<A, I>
{
    /// Create a new Dedup Iterator.
    pub fn new(iter: I) -> Dedup<A, I>
    {
        Dedup{last: None, iter: iter}
    }
}

impl<A: PartialEq, I: Iterator<Item=A>> Iterator for Dedup<A, I>
{
    type Item = A;
    #[inline]
    fn next(&mut self) -> Option<A>
    {
        for elt in self.iter {
            match self.last {
                Some(ref x) if x == &elt => continue,
                None => {
                    self.last = Some(elt);
                    continue;
                }

                ref mut lst => {
                    let ret = mem::replace(lst, Some(elt));
                    return ret
                }
            }
        }
        self.last.take()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>)
    {
        let (lower, upper) = self.iter.size_hint();
        if self.last.is_some() || lower > 0 {
            (1, upper.and_then(|x| x.checked_add(1)))
        } else {
            // they might all be duplicates
            (0, upper)
        }
    }
}


/// A “meta iterator adaptor”. Its closure recives a reference to the iterator
/// and may pick off as many elements as it likes, to produce the next iterator element.
///
/// Iterator element type is **B**, if the return type of **F** is **Option\<B\>**.
#[derive(Clone)]
pub struct Batching<I, F> {
    f: F,
    iter: I,
}

impl<F, I> Batching<I, F> {
    /// Create a new Batching iterator.
    pub fn new(iter: I, f: F) -> Batching<I, F>
    {
        Batching{f: f, iter: iter}
    }
}

impl<A, B, F, I> Iterator for Batching<I, F> where
    I: Iterator<Item=A>,
    F: for<'a> FnMut<(&'a mut I, ), Output=Option<B>>
{
    type Item = B;
    #[inline]
    fn next(&mut self) -> Option<B>
    {
        (self.f)(&mut self.iter)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>)
    {
        // No information about closue behavior
        (0, None)
    }
}

/// Group iterator elements. Consecutive elements that map to the same key ("runs"),
/// are returned as the iterator elements of `GroupBy`.
///
/// Iterator element type is `(K, Vec<A>)`
#[derive(Clone)]
pub struct GroupBy<A, K, I, F> {
    key: F,
    iter: I,
    current_key: Option<K>,
    elts: Vec<A>,
}

impl<A, K, F, I> GroupBy<A, K, I, F> {
    /// Create a new GroupBy iterator.
    pub fn new(iter: I, key: F) -> GroupBy<A, K, I, F>
    {
        GroupBy{key: key, iter: iter, current_key: None, elts: Vec::new()}
    }
}

impl<A, K: PartialEq, F: FnMut(&A) -> K, I: Iterator<Item=A>>
    Iterator for GroupBy<A, K, I, F>
{
    type Item = (K, Vec<A>);
    fn next(&mut self) -> Option<(K, Vec<A>)>
    {
        for elt in self.iter {
            let key = (self.key)(&elt);
            match self.current_key.take() {
                None => {}
                Some(old_key) => if old_key != key {
                    self.current_key = Some(key);
                    let v = mem::replace(&mut self.elts, vec![elt]);
                    return Some((old_key, v))
                },
            }
            self.current_key = Some(key);
            self.elts.push(elt);
        }
        match self.current_key.take() {
            None => None,
            Some(key) => {
                let v = mem::replace(&mut self.elts, Vec::new());
                Some((key, v))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>)
    {
        let (lower, upper) = self.iter.size_hint();
        let stored_count = self.current_key.is_some() as usize;
        let my_upper = upper.and_then(|x| x.checked_add(stored_count));
        if lower > 0 || stored_count > 0 {
            (1, my_upper)
        } else {
            (0, my_upper)
        }
    }
}

/// An iterator adaptor that steps a number elements in the base iterator
/// for each iteration.
///
/// The iterator steps by yielding the next element from the base iterator,
/// then skipping forward *n-1* elements.
#[derive(Clone)]
pub struct Step<I> {
    iter: Fuse<I>,
    skip: usize,
}

impl<I> Step<I>
    where I: Iterator
{
    /// Create a **Step** iterator.
    ///
    /// **Panics** if the step is 0.
    pub fn new(iter: I, step: usize) -> Self
    {
        assert!(step != 0);
        Step{iter: iter.fuse(), skip: step - 1}
    }
}

impl<I> Iterator for Step<I>
    where I: Iterator
{
    type Item = <I as Iterator>::Item;
    #[inline]
    fn next(&mut self) -> Option<<I as Iterator>::Item>
    {
        let elt = self.iter.next();
        self.iter.dropn(self.skip);
        elt
    }

    fn size_hint(&self) -> (usize, Option<usize>)
    {
        let (low, high) = self.iter.size_hint();
        let div = |&: x: usize| {
            if x == 0 {
                0
            } else {
                1 + (x - 1) / (self.skip + 1)
            }
        };
        (div(low), high.map(div))
    }
}

pub struct Merge<T, A, B> where
    T: PartialOrd,
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    a: Peekable<T, A>,
    b: Peekable<T, B>,
}

impl<T, A, B> Merge<T, A, B> where
    T: PartialOrd,
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    /// Create a **Merge** iterator.
    pub fn new(a: A, b: B) -> Merge<T, A, B>
    {
        Merge {
            a: a.peekable(),
            b: b.peekable(),
        }
    }
}

impl<T, A, B> Iterator for Merge<T, A, B> where
    T: PartialOrd,
    A: Iterator<Item = T>,
    B: Iterator<Item = T>,
{
    type Item = T;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a_min, a_max) = self.a.size_hint();
        let (b_min, b_max) = self.b.size_hint();

        let min = a_min.checked_add(b_min).unwrap_or(usize::MAX);
        let max = match (a_max, b_max) {
            (Some(a_min), Some(b_min)) => a_min.checked_add(b_min),
            _ => None,
        };

        (min, max)
    }

    fn next(&mut self) -> Option<T> {
        if match (self.a.peek(), self.b.peek()) {
            (Some(a), Some(b)) => a <= b,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => return None,
        } {
            self.a.next()
        } else {
            self.b.next()
        }
    }
}
