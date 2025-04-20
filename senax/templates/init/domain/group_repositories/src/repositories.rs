#[cfg(any(feature = "mock", test))]
macro_rules! get_emu_repo {
    ($n:ident, $o:ty, $i:ty) => {
        fn $n(&self) -> Box<$o> {
            let mut repo = self._repo.lock().unwrap();
            let repo = repo
                .entry(TypeId::of::<$i>())
                .or_insert_with(|| Box::new(<$i>::default()));
            Box::new(repo.downcast_ref::<$i>().unwrap().clone())
        }
    };
}

// Do not modify below this line. (ModStart)
// Do not modify up to this line. (ModEnd)
@{-"\n"}@
