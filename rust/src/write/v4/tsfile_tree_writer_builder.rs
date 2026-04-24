use std::path::{Path, PathBuf};
use crate::error::TsFileResult;
use crate::write::schema::Schema;
use crate::write::v4::TsFileTreeWriter;

#[derive(Debug)]
pub struct TsFileTreeWriterBuilder {
    path: PathBuf,
    schema: Option<Schema>,
}

impl TsFileTreeWriterBuilder {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        TsFileTreeWriterBuilder { path: path.as_ref().to_path_buf(), schema: None }
    }

    pub fn with_schema(mut self, schema: Schema) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn build(self) -> TsFileResult<TsFileTreeWriter> {
        let mut writer = TsFileTreeWriter::new(self.path)?;
        if let Some(schema) = self.schema {
            for (device, schemas) in schema.measurement_schemas {
                for measurement_schema in schemas {
                    writer.register_timeseries(device.clone(), measurement_schema)?;
                }
            }
        }
        Ok(writer)
    }
}
