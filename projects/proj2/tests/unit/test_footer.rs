use isbd_dbms::format::{ColumnSchema, ColumnType, FileFooter};
use isbd_dbms::format::footer::{serialize_footer, deserialize_footer};

#[test]
fn test_footer_roundtrip() {
    let footer = FileFooter {
        num_rows: 2048,
        num_columns: 2,
        columns: vec![
            ColumnSchema {
                name: "id".to_string(),
                column_type: ColumnType::Int64,
                num_pages: 2,
                page_offsets: vec![5, 1500],
                page_compressed_sizes: vec![100, 200],
                page_row_counts: vec![1024, 1024],
            },
            ColumnSchema {
                name: "active".to_string(),
                column_type: ColumnType::Boolean,
                num_pages: 2,
                page_offsets: vec![115, 1700],
                page_compressed_sizes: vec![128, 128],
                page_row_counts: vec![1024, 1024],
            },
        ],
    };

    let serialized = serialize_footer(&footer);
    let deserialized = deserialize_footer(&serialized).expect("Footer deserialization failed");

    assert_eq!(deserialized.num_rows, footer.num_rows);
    assert_eq!(deserialized.num_columns, footer.num_columns);
    assert_eq!(deserialized.columns.len(), footer.columns.len());

    for (orig, dec) in footer.columns.iter().zip(deserialized.columns.iter()) {
        assert_eq!(orig.name, dec.name);
        assert_eq!(orig.column_type, dec.column_type);
        assert_eq!(orig.num_pages, dec.num_pages);
        assert_eq!(orig.page_offsets, dec.page_offsets);
        assert_eq!(orig.page_compressed_sizes, dec.page_compressed_sizes);
        assert_eq!(orig.page_row_counts, dec.page_row_counts);
    }
}

#[test]
fn test_footer_roundtrip_variable_page_sizes() {
    // Test with pages of different row counts.
    let footer = FileFooter {
        num_rows: 2500,
        num_columns: 1,
        columns: vec![
            ColumnSchema {
                name: "id".to_string(),
                column_type: ColumnType::Int64,
                num_pages: 3,
                page_offsets: vec![5, 1000, 2000],
                page_compressed_sizes: vec![500, 500, 300],
                page_row_counts: vec![1024, 1024, 452],
            },
        ],
    };

    let serialized = serialize_footer(&footer);
    let deserialized = deserialize_footer(&serialized).expect("Footer deserialization failed");

    assert_eq!(deserialized.columns[0].page_row_counts, vec![1024, 1024, 452]);
}
