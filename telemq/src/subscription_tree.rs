use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};
use tokio::sync::RwLock;

use crate::session_state_store::SessionStateStore;
use mqtt_packets::v_3_1_1::topic::{SINGLE_LEVEL_WILD_CARD, WILD_CARD};

type PathStep = String;
type ClientID = String;

#[derive(Debug)]
pub struct SubscriptionTree(SubscriptionNode);

impl SubscriptionTree {
  pub async fn from_session_state_store(state_store: Arc<RwLock<SessionStateStore>>) -> Self {
    let mut tree = SubscriptionTree(SubscriptionNode::new());

    for (_, v) in state_store.read().await.as_inner_data().await {
      for s in v.subscriptions {
        tree.add_subscriber(&s.1.path, v.client_id.clone());
      }
    }

    tree
  }

  pub fn add_subscriber(&mut self, subscription: &[PathStep], connection: ClientID) {
    if subscription.is_empty() {
      // cannot subscribe to "" topic
      // bug in topic parser and topic validator?
      return;
    }
    self.0.add(subscription, connection);
  }

  pub fn find_subscribers(&self, subscription: &[PathStep]) -> HashSet<ClientID> {
    let mut acc = HashSet::new();
    self.0.find(subscription, &mut acc);

    acc
  }

  pub fn remove_subscriber(&mut self, subscription: &[PathStep], connection: ClientID) {
    if subscription.is_empty() {
      // cannot subscribe to "" topic
      // bug in topic parser and topic validator?
      return;
    }
    self.0.remove(subscription, connection);
  }

  pub fn disconnect_subscriber(&mut self, connection: &ClientID) {
    self.0.disconnect(connection);
  }
}

#[derive(Debug)]
pub struct SubscriptionNode {
  connections: HashSet<ClientID>,
  children: HashMap<PathStep, SubscriptionNode>,
}

impl SubscriptionNode {
  fn new() -> SubscriptionNode {
    SubscriptionNode {
      connections: HashSet::new(),
      children: HashMap::new(),
    }
  }

  fn add(&mut self, path: &[PathStep], connection: ClientID) {
    if path.is_empty() {
      self.connections.insert(connection);
      return;
    }

    match self.children.get_mut(&path[0]) {
      Some(ref mut node) => {
        node.add(path.split_at(1).1, connection);
      }
      None => {
        let mut new_node = Self::new();
        new_node.add(path.split_at(1).1, connection);
        self.children.insert(path[0].clone(), new_node);
      }
    }
  }

  // returns a boolean value that suggests if a current node
  // could be deleted
  fn remove(&mut self, path: &[PathStep], connection: ClientID) -> bool {
    if path.is_empty() {
      self.connections.retain(|c| c != &connection);
      return self.connections.is_empty() && self.children.is_empty();
    }

    let can_delete_child = match self.children.get_mut(&path[0]) {
      Some(ref mut node) => node.remove(path.split_at(1).1, connection),
      None => {
        // child not found, so no need to remove
        false
      }
    };

    if can_delete_child {
      self.children.remove(&path[0]);
    }

    return self.connections.is_empty() && self.children.is_empty();
  }

  fn disconnect(&mut self, connection: &ClientID) {
    self.connections.retain(|c| c != connection);
    for child in self.children.values_mut() {
      child.disconnect(connection);
    }
  }

  fn find(&self, path: &[PathStep], acc: &mut HashSet<ClientID>) {
    if path.is_empty() {
      // bug?
      return;
    }

    // exact match
    match self.children.get(&path[0]) {
      Some(ref node) => {
        if path.len() == 1 {
          *acc = &*acc | &node.connections;
        } else {
          node.find(path.split_at(1).1, acc);
        }
      }
      None => {}
    }

    // single level match
    match self.children.get(SINGLE_LEVEL_WILD_CARD) {
      Some(ref node) => {
        if path.len() == 1 {
          *acc = &*acc | &node.connections;
        } else {
          node.find(path.split_at(1).1, acc);
        }
      }
      None => {}
    }

    // wildcard match
    match self.children.get(WILD_CARD) {
      Some(ref node) => {
        *acc = &*acc | &node.connections;
      }
      None => {}
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use maplit::hashmap;
  use mqtt_packets::v_3_1_1::topic::Subscription;

  fn new_tree() -> SubscriptionTree {
    SubscriptionTree(SubscriptionNode::new())
  }

  fn make_addr(n: u16) -> ClientID {
    format!("client_{}", n)
  }

  fn make_hash_set(v: Vec<ClientID>) -> HashSet<ClientID> {
    let mut set = HashSet::with_capacity(v.len());

    for id in v {
      set.insert(id);
    }

    set
  }

  #[test]
  fn add_subscriber() {
    let mut tree = new_tree();

    // empty subscription
    {
      tree.add_subscriber(&vec![], make_addr(1));
      assert!(
        tree.0.children.is_empty(),
        "should do nothing if empty-string topic is used as a new subscription (children)"
      );
      assert!(
        tree.0.connections.is_empty(),
        "should do nothing if empty-string topic is used as a new subscription (connections)"
      );
    }

    // create new node during adding a new subscription
    {
      let subscription =
        Subscription::try_from("a/b").expect("should create new subscription from string");
      tree.add_subscriber(&subscription.path, make_addr(1));

      assert!(
        tree.0.connections.is_empty(),
        "first level should not contain any connections"
      );
      let second_level = tree.0.children.get("a").expect("second level should exist");
      let third_level = second_level
        .children
        .get("b")
        .expect("third level should exist");

      assert_eq!(
        third_level.connections,
        make_hash_set(vec![make_addr(1)]),
        "subscriber should be registered on a third level"
      );

      assert!(
        third_level.children.is_empty(),
        "third level should be the last one"
      );
    }
  }

  #[test]
  fn find_subscribers() {
    // no matches
    {
      let tree_no_matches = SubscriptionTree(SubscriptionNode {
        connections: make_hash_set(vec![]),
        children: hashmap! {
          String::from("a") => SubscriptionNode {
            connections: make_hash_set(vec![make_addr(1)]),
            children: HashMap::new()
          },
          String::from("b") => SubscriptionNode {
            connections: make_hash_set(vec![make_addr(5)]),
            children: hashmap!{
              String::from("c") => SubscriptionNode {
                connections: make_hash_set(vec![make_addr(6)]),
                children: HashMap::new()
              }
            }
          }
        },
      });

      assert_eq!(
        tree_no_matches.find_subscribers(&vec![String::from("c")]),
        make_hash_set(vec![])
      );
    }

    // matches
    {
      let tree = SubscriptionTree(SubscriptionNode {
        connections: make_hash_set(vec![]),
        children: hashmap! {
          String::from("a") => SubscriptionNode {
            connections: make_hash_set(vec![make_addr(1)]),
            children: HashMap::new()
          },
          String::from("+") => SubscriptionNode {
            connections: make_hash_set(vec![make_addr(2)]),
            children: hashmap!{
              String::from("c") => SubscriptionNode {
                connections: make_hash_set(vec![make_addr(4)]),
                children: HashMap::new()
              }
            }
          },
          String::from("#") => SubscriptionNode {
            connections: make_hash_set(vec![make_addr(3)]),
            children: HashMap::new()
          },
          String::from("b") => SubscriptionNode {
            connections: make_hash_set(vec![make_addr(5)]),
            children: hashmap!{
              String::from("c") => SubscriptionNode {
                connections: make_hash_set(vec![make_addr(6)]),
                children: HashMap::new()
              }
            }
          }
        },
      });

      let subscribers = tree.find_subscribers(&vec![String::from("b"), String::from("c")]);

      assert_eq!(subscribers.len(), 3, "number of subscribers");
      assert!(
        subscribers.contains(&make_addr(4)),
        "single level wild card should work"
      );
      assert!(subscribers.contains(&make_addr(3)), "wild card should work");
      assert!(
        subscribers.contains(&make_addr(6)),
        "exact match should work"
      );
    }

    // add + match
    {
      let mut tree = new_tree();
      let sub = vec![String::from("a"), String::from("b")];

      tree.add_subscriber(&sub, make_addr(3));

      let res = tree.find_subscribers(&sub);

      assert_eq!(
        res,
        make_hash_set(vec![make_addr(3)]),
        "should find newly added subscription"
      )
    }
  }

  #[test]
  fn remove_subscriber() {
    // + clean an entire tree
    {
      let mut tree = new_tree();
      let sub = vec![String::from("a"), String::from("b")];

      tree.add_subscriber(&sub, make_addr(3));

      tree.remove_subscriber(&sub, make_addr(3));
    }

    // + clean a sub-tree
    {
      let mut tree = new_tree();
      let sub_1 = vec![String::from("a"), String::from("b")];
      let sub_2 = vec![String::from("a"), String::from("b"), String::from("c")];

      tree.add_subscriber(&sub_1, make_addr(3));
      tree.add_subscriber(&sub_2, make_addr(5));

      tree.remove_subscriber(&sub_1, make_addr(3));
    }
  }
}
