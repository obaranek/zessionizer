//! Frecency score calculation for project sorting.
//!
//! Implements a "frecency" (frequency + recency) algorithm to rank projects based on
//! both how often they are accessed and how recently. This provides a more useful
//! ordering than pure alphabetical or modification-time sorting.
//!
//! The algorithm uses exponential decay with a half-life of 168 hours (1 week),
//! meaning projects accessed a week ago contribute about half their frequency weight
//! to the final score.

use super::models::ProjectRecord;

/// Half-life for exponential decay in hours.
///
/// Projects accessed this many hours ago contribute approximately 50% of their
/// access count to the frecency score. Set to 168 hours (1 week).
const HALF_LIFE_HOURS: f64 = 168.0;

/// Number of seconds per hour for time conversion.
const SECONDS_PER_HOUR: f64 = 3600.0;

/// Calculates the frecency score for a project.
///
/// The score combines frequency (access count) with recency (time since last access)
/// using exponential decay:
///
/// ```text
/// score = access_count × e^(-age_hours / HALF_LIFE_HOURS)
/// ```
///
/// Projects never accessed receive a score based on their access count alone.
/// More recently accessed projects have higher scores due to the recency multiplier.
///
/// # Examples
///
/// ```
/// use crate::storage::{ProjectRecord, calculate_score};
///
/// let mut project = ProjectRecord::new("/home/user/project", "project");
/// project.access_count = 10;
/// project.last_accessed = Some(chrono::Utc::now().timestamp() - 3600); // 1 hour ago
///
/// let now = chrono::Utc::now().timestamp();
/// let score = calculate_score(&project, now);
/// assert!(score > 0.0);
/// assert!(score < 10.0); // Less than pure access count due to time decay
/// ```
#[must_use]
pub fn calculate_score(project: &ProjectRecord, now: i64) -> f64 {
    let access_count = f64::from(project.access_count);

    let recency_multiplier = project.last_accessed.map_or(1.0, |last_accessed| {
        #[allow(clippy::cast_precision_loss)]
        let age_seconds = (now - last_accessed).max(0) as f64;
        let age_hours = age_seconds / SECONDS_PER_HOUR;

        f64::exp(-age_hours / HALF_LIFE_HOURS)
    });

    access_count * recency_multiplier
}

/// Sorts a slice of project records by frecency score in descending order.
///
/// Projects with higher frecency scores (more frequently and recently accessed)
/// appear first in the sorted slice.
///
/// # Parameters
///
/// * `records` - Mutable slice of project records to sort in-place
///
/// # Examples
///
/// ```
/// use crate::storage::{ProjectRecord, sort_by_frecency};
///
/// let mut projects = vec![
///     ProjectRecord::new("/home/user/old-project", "old-project"),
///     ProjectRecord::new("/home/user/new-project", "new-project"),
/// ];
///
/// sort_by_frecency(&mut projects);
/// // projects is now sorted by frecency score (highest first)
/// ```
pub fn sort_by_frecency(records: &mut [ProjectRecord]) {
    let now = chrono::Utc::now().timestamp();
    records.sort_by(|a, b| {
        let score_a = calculate_score(a, now);
        let score_b = calculate_score(b, now);
        score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
    });
}
