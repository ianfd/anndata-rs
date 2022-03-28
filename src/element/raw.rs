use crate::anndata_trait::*;

use std::boxed::Box;
use hdf5::{Result, Group}; 

pub struct RawElem<T: ?Sized> {
    pub dtype: DataType,
    pub(crate) container: DataContainer,
    pub(crate) element: Option<Box<T>>,
}

impl<T> RawElem<T>
where
    T: DataIO,
{
    pub fn read_data(&self) -> T { ReadData::read(&self.container).unwrap() }
}

impl<T> AsRef<RawElem<T>> for RawElem<dyn DataIO>
where
    T: DataIO,
{
    fn as_ref(&self) -> &RawElem<T> {
        if self.dtype == T::dtype() {
            unsafe { &*(self as *const RawElem<dyn DataIO> as *const RawElem<T>) }
        } else {
            panic!(
                "implementation error, cannot convert {:?} to {:?}",
                self.dtype,
                T::dtype(),
            )
        }
    }
}


pub struct RawMatrixElem<T: ?Sized> {
    pub obs_indices: Option<Vec<usize>>,
    pub var_indices: Option<Vec<usize>>,
    pub nrows: usize,
    pub ncols: usize,
    pub inner: RawElem<T>,
}

impl<T> RawMatrixElem<T>
where
    T: DataPartialIO,
{
    pub fn dtype(&self) -> DataType { self.inner.dtype.clone() }

    pub fn new_elem(container: DataContainer) -> Result<Self> {
        let dtype = container.get_encoding_type().unwrap();
        let nrows = get_nrows(&container);
        let ncols = get_ncols(&container);
        let inner = RawElem { dtype, element: None, container };
        Ok(Self { obs_indices: None, var_indices: None, nrows, ncols, inner })
    }

    pub fn read_elem(&self) -> T {
        match self.obs_indices.as_ref() {
            None => match self.var_indices.as_ref() {
                None => ReadData::read(&self.inner.container).unwrap(),
                Some(cidx) => ReadCols::read_columns(
                    &self.inner.container, cidx
                ),
            },
            Some(ridx) => match self.var_indices.as_ref() {
                None => ReadRows::read_rows(&self.inner.container, ridx),
                Some(cidx) => ReadPartial::read_partial(
                    &self.inner.container, ridx, cidx,
                ),
            }
        }
    }

    pub fn write_elem(&self, location: &Group, name: &str) -> Result<()> {
        match &self.inner.element {
            Some(data) => data.write(location, name)?,
            None => self.read_elem().write(location, name)?,
        };
        Ok(())
    }

    // TODO: fix subsetting
    pub fn subset_rows(&self, idx: &[usize]) -> Self {
        for i in idx {
            if *i >= self.nrows {
                panic!("index out of bound")
            }
        }

        let inner = RawElem {
            dtype: self.inner.dtype.clone(),
            container: self.inner.container.clone(),
            element: None,
        };
        Self {
            obs_indices: Some(idx.iter().map(|x| *x).collect()),
            var_indices: self.var_indices.clone(),
            nrows: self.nrows,
            ncols: self.ncols,
            inner,
        }
    }

    pub fn subset_cols(&self, idx: &[usize]) -> Self {
        let inner = RawElem {
            dtype: self.inner.dtype.clone(),
            container: self.inner.container.clone(),
            element: None,
        };
        Self {
            obs_indices: self.obs_indices.clone(),
            var_indices: Some(idx.iter().map(|x| *x).collect()),
            nrows: self.nrows,
            ncols: self.ncols,
            inner,
        }
    }

    pub fn subset(&self, ridx: &[usize], cidx: &[usize]) -> Self {
        let inner = RawElem {
            dtype: self.inner.dtype.clone(),
            container: self.inner.container.clone(),
            element: None,
        };
        Self {
            obs_indices: Some(ridx.iter().map(|x| *x).collect()),
            var_indices: Some(cidx.iter().map(|x| *x).collect()),
            nrows: self.nrows,
            ncols: self.ncols,
            inner,
        }
    }
}

// NOTE: this requires `element` is the last field, as trait object contains a vtable
// at the end: https://docs.rs/vptr/latest/vptr/index.html.
impl<T> AsRef<RawMatrixElem<T>> for RawMatrixElem<dyn DataPartialIO>
where
    T: DataPartialIO,
{
    fn as_ref(&self) -> &RawMatrixElem<T> {
        if self.inner.dtype == T::dtype() {
            unsafe { &*(self as *const RawMatrixElem<dyn DataPartialIO> as *const RawMatrixElem<T>) }
        } else {
            panic!(
                "implementation error, cannot convert {:?} to {:?}",
                self.inner.dtype,
                T::dtype(),
            )
        }
    }
}

impl RawMatrixElem<dyn DataPartialIO>
{
    pub fn new(container: DataContainer) -> Result<Self> {
        let dtype = container.get_encoding_type().unwrap();
        let nrows = get_nrows(&container);
        let ncols = get_ncols(&container);
        let inner = RawElem { dtype, element: None, container };
        Ok(Self { obs_indices: None, var_indices: None, nrows, ncols, inner })
    }

    pub fn read_elem(&self) -> Box<dyn DataPartialIO> {
        match &self.inner.element {
            Some(data) => dyn_clone::clone_box(data.as_ref()),
            None => read_dyn_data_subset(
                &self.inner.container,
                self.obs_indices.as_ref().map(Vec::as_slice),
                self.var_indices.as_ref().map(Vec::as_slice),
            ).unwrap(),
        }
    }

    pub fn write_elem(&self, location: &Group, name: &str) -> Result<()> {
        match &self.inner.element {
            Some(data) => data.write(location, name)?,
            None => self.read_elem().write(location, name)?,
        };
        Ok(())
    }

    // TODO: fix subsetting
    pub fn subset_rows(&self, idx: &[usize]) -> Self {
        for i in idx {
            if *i >= self.nrows {
                panic!("index out of bound")
            }
        }

        let inner = RawElem {
            dtype: self.inner.dtype.clone(),
            container: self.inner.container.clone(),
            element: None,
        };
        Self {
            obs_indices: Some(idx.iter().map(|x| *x).collect()),
            var_indices: self.var_indices.clone(),
            nrows: self.nrows,
            ncols: self.ncols,
            inner,
        }
    }

    pub fn subset_cols(&self, idx: &[usize]) -> Self {
        let inner = RawElem {
            dtype: self.inner.dtype.clone(),
            container: self.inner.container.clone(),
            element: None,
        };
        Self {
            obs_indices: self.obs_indices.clone(),
            var_indices: Some(idx.iter().map(|x| *x).collect()),
            nrows: self.nrows,
            ncols: self.ncols,
            inner,
        }
    }

    pub fn subset(&self, ridx: &[usize], cidx: &[usize]) -> Self {
        let inner = RawElem {
            dtype: self.inner.dtype.clone(),
            container: self.inner.container.clone(),
            element: None,
        };
        Self {
            obs_indices: Some(ridx.iter().map(|x| *x).collect()),
            var_indices: Some(cidx.iter().map(|x| *x).collect()),
            nrows: self.nrows,
            ncols: self.ncols,
            inner,
        }
    }
}