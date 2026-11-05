use std::collections::BTreeMap;

/// Points per player name, listed alphabetically.
#[derive(Debug, Default)]
pub(crate) struct Score(BTreeMap<String, i32>);

impl Score {
    /// Lists a player with zero points unless the player is already listed.
    pub(crate) fn register(&mut self, name: &str) {
        self.0.entry(name.to_string()).or_insert(0);
    }

    /// Gives the player one more point.
    pub(crate) fn increase(&mut self, name: &str) {
        *self.0.entry(name.to_string()).or_insert(0) += 1;
    }

    /// Returns every player with the points, sorted by name.
    pub(crate) fn standings(&self) -> Vec<(String, i32)> {
        self.0
            .iter()
            .map(|(name, points)| (name.clone(), *points))
            .collect()
    }
}

#[cfg(test)]
mod score_tests {
    use super::Score;

    #[test]
    fn registered_players_start_with_zero_points() {
        let mut score = Score::default();
        score.register("anna");

        assert_eq!(score.standings(), vec![("anna".to_string(), 0)]);
    }

    #[test]
    fn registering_again_keeps_points() {
        let mut score = Score::default();
        score.increase("anna");
        score.register("anna");

        assert_eq!(score.standings(), vec![("anna".to_string(), 1)]);
    }

    #[test]
    fn standings_are_sorted_by_name() {
        let mut score = Score::default();
        score.increase("zoe");
        score.increase("anna");
        score.increase("anna");

        assert_eq!(
            score.standings(),
            vec![("anna".to_string(), 2), ("zoe".to_string(), 1)]
        );
    }
}
