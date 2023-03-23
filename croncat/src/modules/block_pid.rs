use std::collections::BTreeMap;
use std::time::Duration;

///
/// Inline Polling Cache
/// We need to poll as close to the block height as possible to be able to make actions during the next block
/// This will only be possible establishing a simple "PID" loop on block height & block timestamps
/// The following is the simple block cache to handle variance in block height/timestamps
/// So our polling stream can validate as soon as possible to the next block finalized
///
#[derive(Default)]
pub struct BlockPid {
    /// k: Block Height, v: Block Timestamp (Nanos)
    pub height: BTreeMap<u64, u128>,

    /// (Block Height, Block Timestamp)
    pub current: (u64, u128),
}

pub type BlockPidDiff = (Duration, u64);

// Default! but not on your loans


impl BlockPid {
    /// compute and return avg_duration
    /// Response: (avg duration, avg variance)
    pub fn compute_avgs(&mut self) -> BlockPidDiff {
        // Remove over threshold, so data doesnt get out of hand
        // TODO: Change!!!!!
        if self.height.len() > 5usize {
            self.height.pop_first();
        }

        // avgs timestamp distances together
        let mut previous: (&u64, &u128) = (&0u64, &0u128);
        let mut diffs: Vec<u128> = vec![];
        for (h, t) in self.height.iter() {
            if previous.0 == &0u64 {
                // simply assign previous
                previous = (h, t);
            } else {
                // find the diff
                if h.saturating_sub(previous.0.to_owned()) != 1 {
                    // Check if missed block height, to make an average of that as well
                    let block_height_sum = h.saturating_sub(previous.0.to_owned());
                    diffs.push(
                        t.saturating_sub(previous.1.to_owned())
                            .saturating_div(block_height_sum.into()),
                    );
                } else {
                    diffs.push(t.saturating_sub(previous.1.to_owned()));
                }
            }
        }

        // The variance of offset from duration
        // Example: Block duration is 5.1seconds, variance ~0.1 seconds
        // value is 0.125 to have enough to always be after block height established
        let mut prev: i32 = 0i32;
        let mut vars: Vec<i32> = vec![];
        for v in diffs.iter() {
            let va = i32::try_from(*v).ok().unwrap();
            if prev == 0i32 {
                // simply assign previous
                prev = va
            } else {
                // find the diff
                vars.push(va - prev);
            }
        }

        let total_dur = diffs.len() as u128;
        let sum_dur: u128 = Iterator::sum(diffs.into_iter());
        let avg_dur = sum_dur.checked_div(total_dur).unwrap_or(0);

        let total_var = vars.len() as i32;
        let sum_var: i32 = Iterator::sum(vars.into_iter());
        let avg_var = sum_var.checked_div(total_var).unwrap_or(0);
        (
            Duration::from_millis(avg_dur.try_into().unwrap()),
            avg_var.abs().try_into().unwrap(),
        )
    }

    /// compute and return duration that lands on or after next block height.
    /// Will do math about: (Block Time + duration + variance) - Current Time
    /// NOTE: timestamp should be in nanos?
    /// TODO: Add optional latency offset
    pub fn get_next(
        &mut self,
        now_timestamp: u128,
        block_height: u64,
        block_timestamp: u128,
    ) -> BlockPidDiff {
        // push into known heights
        self.height.insert(block_height, block_timestamp);
        self.current = (block_height, block_timestamp);

        // compute the avgs
        let (avg_duration, avg_variance) = self.compute_avgs();
        let variance_millis = Duration::from_millis(avg_variance);

        // return duration from maths
        // (block_timestamp + avg_duration + avg_variance) - Duration(epoch time now)
        let now = Duration::from_millis(now_timestamp.try_into().unwrap());
        let block_len = Duration::from_millis(block_timestamp.try_into().unwrap())
            .checked_add(avg_duration)
            .unwrap();
        let block_diff = block_len.checked_add(variance_millis).unwrap();
        let block_offset = if let Some(offset) = block_diff.checked_sub(now) {
            offset
        } else {
            // TODO: take into account latency inside the non-duration
            std::cmp::min(avg_duration, now.checked_sub(block_diff).unwrap())
        };
        (block_offset, avg_variance)
    }
}

#[cfg(test)]
mod tests {
    use crate::modules::block_pid::BlockPid;
    use std::time::Duration;

    #[test]
    fn can_compute_duration() {
        let mut blockpid = BlockPid::default();

        // add somethings
        blockpid.height.insert(1, 1000);
        blockpid.height.insert(2, 2010);
        blockpid.height.insert(3, 3006);
        blockpid.height.insert(4, 4001);
        blockpid.height.insert(5, 4998);
        blockpid.height.insert(6, 5998);
        blockpid.height.insert(7, 7001);
        blockpid.height.insert(8, 8015);
        blockpid.height.insert(9, 9000);

        // lets go
        let (avg_duration, avg_variance) = blockpid.compute_avgs();

        assert_eq!(avg_duration, Duration::new(1, 1000000)); // 1.001s
        assert_eq!(avg_variance, 9u64);
    }

    #[test]
    fn can_compute_next_timestamp() {
        let mut blockpid = BlockPid::default();

        // Start
        let now_millis: u128 = 1678296299935;

        // add somethings
        blockpid.height.insert(1, now_millis.saturating_sub(9000));
        blockpid.height.insert(2, now_millis.saturating_sub(8015));
        blockpid.height.insert(3, now_millis.saturating_sub(7001));
        blockpid.height.insert(4, now_millis.saturating_sub(5998));
        blockpid.height.insert(5, now_millis.saturating_sub(4998));
        blockpid.height.insert(6, now_millis.saturating_sub(4001));
        blockpid.height.insert(7, now_millis.saturating_sub(3006));
        blockpid.height.insert(8, now_millis.saturating_sub(2010));
        blockpid.height.insert(9, now_millis.saturating_sub(1000));

        // lets go
        let (next_duration, next_variance) = blockpid.get_next(now_millis, 10, now_millis);

        assert_eq!(next_variance, 14u64);
        assert_eq!(next_duration, Duration::from_millis(1011),);
    }
}
