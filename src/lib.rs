#![cfg_attr(not(feature = "std"), no_std)]

pub mod alloc;
pub mod boxing;
pub mod dynarray;
pub mod list;
pub mod misc;
pub mod ptr;
pub mod rcobj;
pub mod scope_guard;

#[cfg(test)]
mod tests {
    use crate::{
        alloc::{LDAlloc, StdAlloc},
        dynarray::DynArray,
        list::List,
    };

    #[test]
    fn it_works() {
        let mut allocator = StdAlloc::new();
        let mut ld_allocator = LDAlloc::new(&mut allocator);

        println!("List test:");
        {
            let mut ls = List::<i32>::new(allocator.into_ptr_mut());

            for i in 1..100 {
                if ls.push_back(i).is_none() {
                    panic!("Error inserting element, aborting");
                }
            }

            for i in 1..100 {
                if ls.push_front(i).is_none() {
                    panic!("Error inserting element, aborting");
                }
            }

            for i in ls.begin() {
                println!("{}", i);
            }
        }

        println!("Vec test:");
        {
            let mut a = DynArray::<i32>::new(allocator.into_ptr_mut());

            for i in 1..100 {
                if a.push_back(i).is_none() {
                    panic!("Error inserting element, aborting");
                }
            }

            for i in 1..100 {
                if a.push_front(i).is_none() {
                    panic!("Error inserting element, aborting");
                }
            }

            for i in 1i32..100i32 {
                if a.insert(i as usize, i).is_none() {
                    panic!("Error inserting element, aborting");
                }
            }

            for i in a.begin() {
                println!("{}", i);
            }
        }

        // allocator.dump_allocated_blocks();
    }
}
