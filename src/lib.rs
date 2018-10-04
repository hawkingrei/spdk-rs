#[warn(non_camel_case_types)]

pub mod nvme; 
pub mod env; 
pub mod stdinc;
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
