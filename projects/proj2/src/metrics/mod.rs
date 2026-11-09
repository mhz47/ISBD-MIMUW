use crate::format::{Batch, ColumnBatch};

#[derive(Debug)]
pub struct Int64Metrics {
    pub name: String,
    pub average: f64,
}

#[derive(Debug)]
pub struct BooleanMetrics {
    pub name: String,
    pub entropy: f64,
}

#[derive(Debug)]
pub struct VarcharMetrics {
    pub name: String,
    pub char_count: usize,
}

/// Calculate the average of INT64 values.
pub fn calculate_int64_average(values: &[i64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let sum: i128 = values.iter().map(|&v| v as i128).sum();
    sum as f64 / values.len() as f64
}

/// Count the total number of ASCII characters in a VARCHAR column.
pub fn calculate_varchar_char_count(values: &[String]) -> usize {
    values.iter().map(|s| s.as_bytes().iter().filter(|&&b| b < 128).count()).sum()
}

/// Calculate binary entropy:
/// H(p) = -p*log2(p) - (1-p)*log2(1-p)
/// where p = true_count / total_count.
pub fn calculate_entropy_from_counts(true_count: usize, total_count: usize) -> f64 {
    if total_count == 0 {
        return 0.0;
    }
    let p = true_count as f64 / total_count as f64;
    if p == 0.0 || p == 1.0 {
        return 0.0;
    }
    -p * p.log2() - (1.0 - p) * (1.0 - p).log2()
}

/// Calculate metrics by streaming over batches.
pub fn calculate_metrics_stream<I>(batch_iter: I) -> Result<(Vec<Int64Metrics>, Vec<BooleanMetrics>, Vec<VarcharMetrics>), String>
where
    I: Iterator<Item = Result<Batch, String>>,
{
    let mut int64_names: Vec<String> = Vec::with_capacity(1);
    let mut int64_sum: Vec<i128> = Vec::with_capacity(1);
    let mut int64_count: Vec<usize> = Vec::with_capacity(1);
    
    let mut boolean_names: Vec<String> = Vec::with_capacity(1);
    let mut boolean_true_count: Vec<usize> = Vec::with_capacity(1);
    let mut boolean_total_count: Vec<usize> = Vec::with_capacity(1);
    
    let mut varchar_names: Vec<String> = Vec::with_capacity(1);
    let mut varchar_char_count: Vec<usize> = Vec::with_capacity(1);
    
    for batch_result in batch_iter {
        let batch = batch_result?;
        
        for col in &batch.columns {
            match col {
                ColumnBatch::Int64(c) => {
                    let idx = find_or_add_column(
                        &mut int64_names, &c.name,
                        || { int64_sum.push(0); int64_count.push(0); }
                    );
                    int64_sum[idx] += c.values.iter().map(|&v| v as i128).sum::<i128>();
                    int64_count[idx] += c.values.len();
                }
                ColumnBatch::Boolean(c) => {
                    let idx = find_or_add_column(
                        &mut boolean_names, &c.name,
                        || { boolean_true_count.push(0); boolean_total_count.push(0); }
                    );
                    boolean_true_count[idx] += c.values.iter().filter(|&&v| v).count();
                    boolean_total_count[idx] += c.values.len();
                }
                ColumnBatch::Varchar(c) => {
                    let idx = find_or_add_column(
                        &mut varchar_names, &c.name,
                        || { varchar_char_count.push(0); }
                    );
                    varchar_char_count[idx] += c.values.iter()
                        .map(|s| s.as_bytes().iter().filter(|&&b| b < 128).count())
                        .sum::<usize>();
                }
            }
        }
    }

    /// Find the index of a column by name, or add it and return the new index.
    fn find_or_add_column(names: &mut Vec<String>, name: &str, init: impl FnOnce()) -> usize {
        if let Some(pos) = names.iter().position(|n| n == name) {
            pos
        } else {
            let idx = names.len();
            names.push(name.to_string());
            init();
            idx
        }
    }
    
    let int64_metrics: Vec<Int64Metrics> = int64_names.into_iter().enumerate().map(|(i, name)| {
        let average = if int64_count[i] == 0 {
            0.0
        } else {
            int64_sum[i] as f64 / int64_count[i] as f64
        };
        Int64Metrics { name, average }
    }).collect();
    
    let boolean_metrics: Vec<BooleanMetrics> = boolean_names.into_iter().enumerate().map(|(i, name)| {
        let entropy = calculate_entropy_from_counts(boolean_true_count[i], boolean_total_count[i]);
        BooleanMetrics { name, entropy }
    }).collect();
    
    let varchar_metrics: Vec<VarcharMetrics> = varchar_names.into_iter().enumerate().map(|(i, name)| {
        VarcharMetrics { name, char_count: varchar_char_count[i] }
    }).collect();
    
    Ok((int64_metrics, boolean_metrics, varchar_metrics))
}
