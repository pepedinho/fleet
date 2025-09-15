use core_lib::daemon::utiles::extract_repo_path;
use pretty_assertions::assert_eq;

#[test]
fn test_extract_repo_path() -> anyhow::Result<()> {
    let cases: Vec<(&str, Result<&str, &str>)> = vec![
        // Cas valides
        ("git@github.com:pepedinho/fleet.git", Ok("pepedinho/fleet")),
        (
            "ssh://git@github.com/pepedinho/fleet.git",
            Ok("pepedinho/fleet"),
        ),
        (
            "https://github.com/pepedinho/fleet.git",
            Ok("pepedinho/fleet"),
        ),
        (
            "https://user:token@github.com/pepedinho/fleet.git",
            Ok("pepedinho/fleet"),
        ),
        (
            "git://github.com/pepedinho/fleet.git",
            Ok("pepedinho/fleet"),
        ),
        ("https://github.com/pepedinho/fleet", Ok("pepedinho/fleet")),
        ("git@gitlab.com:group/repo.git", Ok("group/repo")),
        (
            "git@gitlab.com:group/subgroup/repo.git",
            Ok("group/subgroup/repo"),
        ),
        ("https://gitlab.com/group/repo.git", Ok("group/repo")),
        (
            "https://gitlab.com/group/subgroup/repo.git",
            Ok("group/subgroup/repo"),
        ),
        (
            "git@github.com:User-Name/Repo_Name.git",
            Ok("User-Name/Repo_Name"),
        ),
        ("git@github.com:repo.git", Err("Incorrect remote path")),
        (
            "https://github.com",
            Err("Failed to extract repo remote path"),
        ),
        ("", Err("empty remote")),
    ];

    for (input, expected) in cases {
        let result = extract_repo_path(input);

        match expected {
            Ok(expected_str) => {
                assert_eq!(result?, expected_str, "Failed on case: {input}");
            }
            Err(_) => {
                assert!(
                    result.is_err(),
                    "Expected error but got Ok on case: {input}"
                );
            }
        }
    }

    Ok(())
}
