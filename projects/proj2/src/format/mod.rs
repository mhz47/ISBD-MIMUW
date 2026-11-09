pub mod footer;
pub mod serializer;
pub mod deserializer;

pub use serializer::serialize_to_file;
pub use deserializer::BatchIterator;

// Maximum number of rows per page.
pub const PAGE_SIZE: usize = 1024;

// In-memory representation of a batch of columnar data (SoA).
// A batch may contain any number of rows. If larger than PAGE_SIZE, the serializer
// will split it into multiple pages; if smaller, it becomes a single page with fewer rows.
#[derive(Debug, Clone)]
pub struct Batch {
    pub columns: Vec<ColumnBatch>,
    pub num_rows: usize,
}


// A single column's data within a batch.
#[derive(Debug, Clone)]
pub enum ColumnBatch {
    Int64(Int64ColumnBatch),
    Boolean(BooleanColumnBatch),
    Varchar(VarcharColumnBatch),
}

#[derive(Debug, Clone)]
pub struct Int64ColumnBatch {
    pub name: String,
    pub values: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct BooleanColumnBatch {
    pub name: String,
    pub values: Vec<bool>,
}

#[derive(Debug, Clone)]
pub struct VarcharColumnBatch {
    pub name: String,
    pub values: Vec<String>,
}


// Schema of a single column stored in the file footer.
#[derive(Debug, Clone)]
pub struct ColumnSchema {
    pub name: String,
    pub column_type: ColumnType,
    pub num_pages: u32,
    pub page_offsets: Vec<u64>,             // File offset of each page's compressed data.
    pub page_compressed_sizes: Vec<u64>,    // Compressed size of each page.
    pub page_row_counts: Vec<u32>,          // Row count of each page.
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColumnType {
    Int64,
    Boolean,
    Varchar,
}


// File footer with metadata (stored at the end of the file).
#[derive(Debug, Clone)]
pub struct FileFooter {
    pub num_rows: u64,
    pub num_columns: u64,
    pub columns: Vec<ColumnSchema>,
}


// Row-level access interface for Batch
impl Batch {
    // Get a clone of the value at the given column and row index.
    pub fn get(&self, column_idx: usize, row_idx: usize) -> Option<CellValue> {
        self.columns.get(column_idx).and_then(|col| col.get(row_idx))
    }

    // Set a value at the given column and row index.
    pub fn set(&mut self, column_idx: usize, row_idx: usize, value: CellValue) -> Result<(), String> {
        match self.columns.get_mut(column_idx) {
            Some(col) => col.set(row_idx, value),
            None => Err(format!("Column index {} out of bounds", column_idx)),
        }
    }
}

impl ColumnBatch {
    // Get a clone of the value at the given row index.
    fn get(&self, row_idx: usize) -> Option<CellValue> {
        match self {
            ColumnBatch::Int64(col) => col.values.get(row_idx).map(|&v| CellValue::Int64(v)),
            ColumnBatch::Boolean(col) => col.values.get(row_idx).map(|&v| CellValue::Boolean(v)),
            ColumnBatch::Varchar(col) => col.values.get(row_idx).cloned().map(CellValue::Varchar),
        }
    }

    // Set a value at the given row index.
    fn set(&mut self, row_idx: usize, value: CellValue) -> Result<(), String> {
        match (self, value) {
            (ColumnBatch::Int64(col), CellValue::Int64(v)) => {
                if row_idx < col.values.len() {
                    col.values[row_idx] = v;
                    Ok(())
                } else {
                    Err("Row index out of bounds".into())
                }
            }
            (ColumnBatch::Boolean(col), CellValue::Boolean(v)) => {
                if row_idx < col.values.len() {
                    col.values[row_idx] = v;
                    Ok(())
                } else {
                    Err("Row index out of bounds".into())
                }
            }
            (ColumnBatch::Varchar(col), CellValue::Varchar(v)) => {
                if row_idx < col.values.len() {
                    col.values[row_idx] = v;
                    Ok(())
                } else {
                    Err("Row index out of bounds".into())
                }
            }
            _ => Err("Type mismatch".into()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ColumnBatch::Int64(col) => &col.name,
            ColumnBatch::Boolean(col) => &col.name,
            ColumnBatch::Varchar(col) => &col.name,
        }
    }

    pub fn column_type(&self) -> ColumnType {
        match self {
            ColumnBatch::Int64(_) => ColumnType::Int64,
            ColumnBatch::Boolean(_) => ColumnType::Boolean,
            ColumnBatch::Varchar(_) => ColumnType::Varchar,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Int64(i64),
    Boolean(bool),
    Varchar(String),
}

