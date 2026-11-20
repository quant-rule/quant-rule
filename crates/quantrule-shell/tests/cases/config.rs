//! Configuration command output tests.

use crate::cases::assertions::{assert_success, stdout};
use crate::support::{Fixture, config_hash};

#[test]
fn config_list_and_show_print_validity_without_inventing_a_hash() {
    let fixture = Fixture::start();
    let list = fixture.run(&["config", "list"]);
    assert_success(&list);
    assert_eq!(
        stdout(&list),
        format!(
            "configs: 2\ncfg_abc name=run origin=repository valid=true message=none hash={}\n\
             cfg_bad name=broken origin=upload valid=false message=portfolio.initial_cash must be positive hash=none\n",
            config_hash()
        )
    );

    let show = fixture.run(&["config", "show", "cfg_bad"]);
    assert_success(&show);
    assert_eq!(
        stdout(&show),
        "config_id: cfg_bad\nname: broken\norigin: upload\nvalid: false\n\
         message: portfolio.initial_cash must be positive\nconfig_hash: none\n"
    );
}
