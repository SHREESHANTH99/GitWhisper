use std::fs;
use std::os::unix::fs::PermissionsExt;

pub fn install_hooks() {

    let pre_commit = ".git/hooks/pre-commit";
    let post_commit = ".git/hooks/post-commit";

    fs::write(pre_commit, "commitlens precommit").unwrap();
    fs::write(post_commit, "commitlens postcommit").unwrap();

    let perm = fs::Permissions::from_mode(0o755);

    fs::set_permissions(pre_commit, perm.clone()).unwrap();
    fs::set_permissions(post_commit, perm).unwrap();

    println!("Hooks installed");
}