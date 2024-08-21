use std::ops::Deref;
use std::sync::OnceLock;

pub struct LazyLock<T> {
    cell: OnceLock<T>,
    init: fn() -> T,
}

impl<T> LazyLock<T> {
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            cell: OnceLock::new(),
            init,
        }
    }
}

impl<T> Deref for LazyLock<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &'_ T {
        self.cell.get_or_init(self.init)
    }
}
