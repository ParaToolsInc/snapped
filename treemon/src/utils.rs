use anyhow::Result;
use histogram::Histogram;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    net::TcpStream,
};
/// A struct representing a histogram with additional metadata.
///
/// This struct contains a vector of buckets, each represented as a tuple
/// of `(start, end, count)`, where `start` and `end` are the bounds of the bucket
/// and `count` is the number of values in that range. It also includes the
/// mean value of the histogram and a timestamp.
#[derive(Serialize, Deserialize, Debug)]
pub struct TbonHistogram {
    /// A vector of buckets, each represented as a tuple of `(start, end, count)`.
    pub buckets: Vec<(u64, u64, u64)>,

    /// The mean value of the histogram.
    pub mean: f64,

    /// A timestamp associated with this histogram.
    pub ts: f64,
}

impl TbonHistogram {
    /// Creates a new `TbonHistogram` instance from a given `Histogram`.
    ///
    /// This function takes a `Histogram` and a timestamp, and returns a new
    /// `TbonHistogram` instance. It calculates the mean value of the histogram
    /// by summing up the weighted values of each bucket.
    ///
    /// # Arguments
    ///
    /// * `h`: The input `Histogram`.
    /// * `ts`: The timestamp to associate with this histogram.
    pub fn from_hist(h: Histogram, ts: f64) -> TbonHistogram {
        let mut buckets: Vec<(u64, u64, u64)> = Vec::new();

        // Iterate over the buckets in the input histogram and filter out
        // empty ones.
        for b in h.into_iter() {
            if b.count() != 0 {
                buckets.push((b.start(), b.end(), b.count()));
            }
        }

        // Calculate the total count of values in the histogram.
        let total: f64 = buckets.iter().map(|(_, _, c)| *c as f64).sum();

        // Initialize a variable to accumulate the weighted sum of values.
        let mut weighted_sum: f64 = 0.0;

        // Iterate over the buckets and calculate their weighted contributions
        // to the mean value.
        for (start, end, count) in buckets.iter() {
            let mid = (*start as f64 + *end as f64) / 2.0;
            weighted_sum += mid * *count as f64;
        }

        // Create a new `TbonHistogram` instance with the calculated mean value.
        TbonHistogram {
            buckets,
            mean: weighted_sum / total,
            ts,
        }
    }
}

pub fn read_command_from_sock(sock: &mut TcpStream) -> Result<Vec<u8>> {
    /* First read size */
    let mut size_buf = [0; std::mem::size_of::<usize>()];
    sock.read_exact(&mut size_buf)?;
    let size = usize::from_le_bytes(size_buf);

    /* Then read data of given size */
    let mut data = vec![0; size];
    sock.read_exact(&mut data)?;

    Ok(data)
}

pub fn write_command_to_sock(sock: &mut TcpStream, command: &[u8]) -> Result<()> {
    let size = command.len();

    /* Write the size of the data */
    sock.write_all(&(size.to_le_bytes()))?;

    /* Then write the data itself */
    sock.write_all(command)?;

    Ok(())
}
