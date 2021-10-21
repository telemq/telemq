use serde::{Deserialize, Serialize};

/// Multi-level wild card.
pub const WILD_CARD: &'static str = "#";

/// Single level wild card.
pub const SINGLE_LEVEL_WILD_CARD: &'static str = "+";

/// Topic begins with wild card.
pub const TOPIC_LEVEL_SEPARATOR: &'static str = "/";

/// System prefix
pub const SYSTEM_PREFIX: &'static str = "$";

/// Number of first bytes that describe a length of a topic.
pub const TOPIC_BYTES_LEN: usize = 2;

/// Topic structure should be used for publishing purposes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Topic {
    pub original: String,
    pub path: Vec<String>,
}

impl Topic {
    /// It tries to convert an argument to a `Topic`. The method returns
    /// `Err` if a string topic contains a protocol violation. In this case
    /// a connection should be closed by the Server.
    pub fn try_from<T: AsRef<str>>(topic_string: T) -> std::io::Result<Self> {
        let topic_name_ref = topic_string.as_ref();

        let contains_wild_card =
            topic_name_ref.contains(WILD_CARD) || topic_name_ref.contains(SINGLE_LEVEL_WILD_CARD);

        if contains_wild_card {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Codec: Published topic name cannot contain wildcard symbols"),
            ));
        }

        let path = topic_name_ref
            .split(TOPIC_LEVEL_SEPARATOR)
            .map(|p| p.to_string())
            .collect();

        Ok(Topic {
            original: topic_name_ref.into(),
            path,
        })
    }

    pub fn make_from_string<T: AsRef<str>>(topic_string: T) -> Self {
        let topic_name_ref = topic_string.as_ref();

        let path = topic_name_ref
            .split(TOPIC_LEVEL_SEPARATOR)
            .map(|p| p.to_string())
            .collect();

        Topic {
            original: topic_name_ref.into(),
            path,
        }
    }

    /// It validates current `Topic`.
    ///
    /// It returns `false` if current topic is not a protocol violation
    /// but still is not valid. If `is_valid` returns `false` connection should
    /// not be closed but a topic should not be registered for publishing by the Server.
    pub fn is_valid(&self) -> bool {
        !self.original.is_empty()
    }
}

/// Client subscription
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Subscription {
    pub original: String,
    pub path: Vec<String>,
}

impl Subscription {
    /// It tries to convert an argument to `Subscription`. If this method returns
    /// `Err` then such subscription is a protocol violation and connection
    /// should be closed by the Server.
    pub fn try_from<T: AsRef<str>>(t: T) -> std::io::Result<Self> {
        let original: String = t.as_ref().into();
        let path: Vec<String> = original
            .split(TOPIC_LEVEL_SEPARATOR)
            .map(|p| p.to_string())
            .collect();
        Ok(Subscription { original, path })
    }

    /// It validates current `Subscription`.
    ///
    /// It returns `false` if current subscription is not a protocol violation
    /// but still is not valid. If `is_valid` returns `false` connection should
    /// not be closed but a subscription should not be registered by the Server.
    pub fn is_valid(&self) -> bool {
        let ref subscription_string = self.original;

        if subscription_string.is_empty() {
            return false;
        }

        Subscription::wild_card_validity(subscription_string)
            && Subscription::single_level_validity(subscription_string)
    }

    /// It returns `true` if topic matches current subscription and `false` otherwise.
    pub fn topic_matches(&self, topic: &Topic) -> bool {
        return topics_match(&topic.path, &self.path);
    }

    fn wild_card_validity(subscription: &str) -> bool {
        if !subscription.contains(WILD_CARD) || subscription == WILD_CARD {
            return true;
        }

        let ends_with_pattern = format!("{}{}", TOPIC_LEVEL_SEPARATOR, WILD_CARD);

        if !subscription.ends_with(ends_with_pattern.as_str()) {
            return false;
        }

        !subscription
            .trim_end_matches(ends_with_pattern.as_str())
            .contains(WILD_CARD)
    }

    fn single_level_validity(subscription: &str) -> bool {
        subscription
            .split(TOPIC_LEVEL_SEPARATOR)
            .all(|level| !level.contains(SINGLE_LEVEL_WILD_CARD) || level == SINGLE_LEVEL_WILD_CARD)
    }
}

pub fn topics_match(left: &Vec<String>, right: &Vec<String>) -> bool {
    for (i, p) in left.iter().enumerate() {
        match right.get(i) {
            Some(pattern) => {
                if pattern == WILD_CARD {
                    return i != 0 || !p.starts_with(SYSTEM_PREFIX);
                }

                if i == 0 && p.starts_with(SYSTEM_PREFIX) && pattern == SINGLE_LEVEL_WILD_CARD {
                    return false;
                }

                if pattern != p && pattern != SINGLE_LEVEL_WILD_CARD {
                    return false;
                }
            }
            None => return false,
        }
    }

    true
}

#[cfg(test)]
mod topic_tests {
    use super::*;

    #[test]
    fn try_from_with_valid_topic() {
        let valid_string = String::from("some valid topic");

        let topic = Topic::try_from(&valid_string)
            .expect("should convert valid string into Topic without errors");

        assert_eq!(
            topic.original, valid_string,
            "should properly conver string into Topic"
        )
    }

    #[test]
    fn can_be_published() {
        assert_eq!(Topic::try_from("some").unwrap().is_valid(), true);
        assert_eq!(Topic::try_from("").unwrap().is_valid(), false);
    }
}

#[cfg(test)]
mod subscription_tests {
    use super::*;

    #[test]
    fn try_from() {
        let sub =
            Subscription::try_from("topic").expect("should create subscription without errors");
        assert_eq!(sub, Subscription::try_from("topic").unwrap());
    }

    #[test]
    fn is_valid() {
        {
            let sub = Subscription::try_from("").unwrap();

            assert_eq!(
                sub.is_valid(),
                false,
                "empty subscription should be invalid"
            );
        }

        {
            let sub = Subscription::try_from("abc/def ghi").unwrap();

            assert_eq!(
                sub.is_valid(),
                true,
                "multi-level subscription should be valid"
            );
        }

        {
            let sub = Subscription::try_from("#").unwrap();

            assert_eq!(sub.is_valid(), true, "# subscription should be valid");
        }

        {
            let sub = Subscription::try_from("abc de/#").unwrap();

            assert_eq!(
                sub.is_valid(),
                true,
                "multi level wildcard subscription should be valid"
            );
        }

        {
            let sub = Subscription::try_from("abc/#/ de/#").unwrap();

            assert_eq!(
                sub.is_valid(),
                false,
                "multi level wildcard subscription should be invalid"
            );
        }

        {
            let sub = Subscription::try_from("+").unwrap();

            assert_eq!(
                sub.is_valid(),
                true,
                "single level wildcard subscription should be valid"
            );
        }

        {
            let sub = Subscription::try_from("+/asd basd").unwrap();

            assert_eq!(
                sub.is_valid(),
                true,
                "subscription starting with single level wildcard should be valid"
            );
        }

        {
            let sub = Subscription::try_from("asd basd/+").unwrap();

            assert_eq!(
                sub.is_valid(),
                true,
                "subscription ends with single level wildcard should be valid"
            );
        }

        {
            let sub = Subscription::try_from("asd basd/+/werwi sdfj").unwrap();

            assert_eq!(
                sub.is_valid(),
                true,
                "subscription with single level wildcard should be valid"
            );
        }

        {
            let sub = Subscription::try_from("asd basd/+/werwi sdfj/#").unwrap();

            assert_eq!(
                sub.is_valid(),
                true,
                "subscription with single and multi level wildcard should be valid"
            );
        }

        {
            let sub = Subscription::try_from("asd basd+/werwi sdfj/+").unwrap();

            assert_eq!(sub.is_valid(), false, "subscription should be invalid (1)");

            let sub = Subscription::try_from("asd basd+").unwrap();

            assert_eq!(sub.is_valid(), false, "subscription should be invalid (2)");
        }
    }

    #[test]
    fn matches_topic() {
        // Vec<(Subscription, Vec<Topic>, Vec<Topic>)>
        let cases = vec![
            // # patterns
            (
                "sport/tennis/player1/#",
                vec![
                    "sport/tennis/player1",
                    "sport/tennis/player1/ranking",
                    "sport/tennis/player1/score/wimbledon",
                ],
                vec![
                    "sport/tennis/player2",
                    "sport/tennis2/player1",
                    "sport1/tennis/player1",
                ],
            ),
            ("sport/#", vec!["sport"], vec!["sport2"]),
            ("#", vec!["sport", "sport2"], vec!["$", "$SYS"]),
            // + patterns
            (
                "+/abc",
                vec!["some/abc", "other/abc"],
                vec!["$", "$SYS", "some/other"],
            ),
            (
                "abc/+/def",
                vec!["abc/xyz/def", "abc/zyx/def"],
                vec!["ab/xyz/def", "abc/xyz/df"],
            ),
            (
                "abc/+",
                vec!["abc/def", "abc/xyz"],
                vec!["ab/xyz", "bc/xyz", "abc/def/ghi"],
            ),
            (
                "+/+",
                vec!["a/b", "c/d", "/x", "/y", "x/", "y/"],
                vec!["abc/def/ghi", "$SYS/abc"],
            ),
            // # + patterns
            (
                "+/abc/#",
                vec!["a/abc", "aa/abc/d", "aa/abc/d/e"],
                vec!["$SYS/abc/d", "a/b/abc/d"],
            ),
        ];

        for case in cases {
            let subscription = Subscription::try_from(case.0).unwrap();

            for positive_topic in case.1 {
                let topic = Topic::try_from(positive_topic).unwrap();

                assert_eq!(
                    subscription.topic_matches(&topic),
                    true,
                    "subscription {:?} should match {:?}",
                    subscription.original,
                    topic.original
                );
            }

            for negative_topic in case.2 {
                let topic = Topic::try_from(negative_topic).unwrap();

                assert_eq!(
                    subscription.topic_matches(&topic),
                    false,
                    "subscription {:?} should not match {:?}",
                    subscription.original,
                    topic.original
                );
            }
        }
    }
}
