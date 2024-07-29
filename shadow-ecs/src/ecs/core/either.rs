pub enum Either<L, R> {
    Left(L),
    Right(R),
}

pub fn left<L, R>(left: L) -> Either<L, R> {
    Either::Left(left)
}

pub fn right<L, R>(right: R) -> Either<L, R> {
    Either::Right(right)
}

impl<L, R> Either<L, R> {
    pub fn left(&self) -> Option<&L> {
        match self {
            Either::Left(l) => Some(l),
            _ => None,
        }
    }

    pub fn right(&self) -> Option<&R> {
        match self {
            Either::Right(r) => Some(r),
            _ => None,
        }
    }

    pub fn left_mut(&mut self) -> Option<&mut L> {
        match self {
            Either::Left(l) => Some(l),
            _ => None,
        }
    }

    pub fn right_mut(&mut self) -> Option<&mut R> {
        match self {
            Either::Right(r) => Some(r),
            _ => None,
        }
    }

    pub fn map_left<U>(self, f: impl FnOnce(L) -> U) -> Either<U, R> {
        match self {
            Either::Left(l) => Either::Left(f(l)),
            Either::Right(r) => Either::Right(r),
        }
    }

    pub fn map_right<U>(self, f: impl FnOnce(R) -> U) -> Either<L, U> {
        match self {
            Either::Left(l) => Either::Left(l),
            Either::Right(r) => Either::Right(f(r)),
        }
    }

    pub fn as_ref(&self) -> Either<&L, &R> {
        match self {
            Either::Left(l) => Either::Left(l),
            Either::Right(r) => Either::Right(r),
        }
    }

    pub fn as_mut(&mut self) -> Either<&mut L, &mut R> {
        match self {
            Either::Left(l) => Either::Left(l),
            Either::Right(r) => Either::Right(r),
        }
    }

    pub fn is_left(&self) -> bool {
        matches!(self, Either::Left(_))
    }

    pub fn is_right(&self) -> bool {
        matches!(self, Either::Right(_))
    }
}

impl<L: Default, R: Default> Default for Either<L, R> {
    fn default() -> Self {
        Either::Left(Default::default())
    }
}

impl<L: Clone, R: Clone> Clone for Either<L, R> {
    fn clone(&self) -> Self {
        match self {
            Either::Left(l) => Either::Left(l.clone()),
            Either::Right(r) => Either::Right(r.clone()),
        }
    }
}

impl<L: Copy, R: Copy> Copy for Either<L, R> {}

impl<L: PartialEq, R: PartialEq> PartialEq for Either<L, R> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Either::Left(l1), Either::Left(l2)) => l1 == l2,
            (Either::Right(r1), Either::Right(r2)) => r1 == r2,
            _ => false,
        }
    }
}

impl<L: Eq, R: Eq> Eq for Either<L, R> {}

impl<L: std::fmt::Debug, R: std::fmt::Debug> std::fmt::Debug for Either<L, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Either::Left(l) => write!(f, "Left({:?})", l),
            Either::Right(r) => write!(f, "Right({:?})", r),
        }
    }
}

impl<L: std::fmt::Display, R: std::fmt::Display> std::fmt::Display for Either<L, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Either::Left(l) => write!(f, "{}", l),
            Either::Right(r) => write!(f, "{}", r),
        }
    }
}

impl<L, R> From<Result<L, R>> for Either<L, R> {
    fn from(result: Result<L, R>) -> Self {
        match result {
            Ok(l) => Either::Left(l),
            Err(r) => Either::Right(r),
        }
    }
}

impl<L, R> From<Either<L, R>> for Result<L, R> {
    fn from(either: Either<L, R>) -> Self {
        match either {
            Either::Left(l) => Ok(l),
            Either::Right(r) => Err(r),
        }
    }
}
