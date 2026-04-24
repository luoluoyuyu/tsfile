// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.

//! Point reader over decoded time-value pairs.

use crate::read::time_value_pair::TimeValuePair;

pub trait PointReader {
    fn has_next(&self) -> bool;
    fn next(&mut self) -> Option<TimeValuePair>;
}

#[derive(Debug, Clone)]
pub struct VecPointReader {
    points: Vec<TimeValuePair>,
    cursor: usize,
}

impl VecPointReader {
    pub fn new(points: Vec<TimeValuePair>) -> Self {
        VecPointReader { points, cursor: 0 }
    }

    pub fn reset(&mut self) {
        self.cursor = 0;
    }
}

impl PointReader for VecPointReader {
    fn has_next(&self) -> bool {
        self.cursor < self.points.len()
    }

    fn next(&mut self) -> Option<TimeValuePair> {
        if !self.has_next() {
            return None;
        }
        let point = self.points[self.cursor].clone();
        self.cursor += 1;
        Some(point)
    }
}
