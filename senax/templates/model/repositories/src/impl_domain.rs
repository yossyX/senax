macro_rules! get_repo {
    ($n:ident, $o:ty, $i:ty) => {
        fn $n(&self) -> Box<$o> {
            Box::new(<$i>::new(Arc::clone(&self._conn)))
        }
    };
}

// Do not modify below this line. (ModStart)
// Do not modify up to this line. (ModEnd)
@{-"\n"}@