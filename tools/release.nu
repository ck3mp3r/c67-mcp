# Release workflow helper functions

# Calculate semantic version based on latest tag and current version
export def calculate-semver [latest_tag_version: string current_version: string] {
  # If no latest tag (empty string), use current version as-is
  if ($latest_tag_version | is-empty) {
    return $current_version
  }

  let current_parts = ($current_version | split row ".")
  let tag_parts = ($latest_tag_version | split row ".")

  let current_major = ($current_parts.0 | into int)
  let current_minor = ($current_parts.1 | into int)
  let current_patch = ($current_parts.2 | into int)

  let tag_major = ($tag_parts.0 | into int)
  let tag_minor = ($tag_parts.1 | into int)
  let tag_patch = ($tag_parts.2 | into int)

  # If major or minor version changed, use current version as-is
  if $current_major != $tag_major or $current_minor != $tag_minor {
    return $current_version
  }

  # If current version is same as latest tag, increment patch
  if $current_version == $latest_tag_version {
    let new_patch = $current_patch + 1
    return $"($current_major).($current_minor).($new_patch)"
  }

  # Otherwise use current version as-is
  $current_version
}

# Get latest git tag version (without 'v' prefix)
export def get-latest-tag [] {
  let result = (^git describe --tags --abbrev=0 | complete)
  if $result.exit_code == 0 {
    $result.stdout | str trim | str replace --regex "^v" ""
  } else {
    ""
  }
}

# Create and checkout release branch, pass version through pipeline
export def create-release-branch [] {
  let version = $in
  let branch_name = $"release/($version)"

  # Check if branch already exists locally
  let local_branch_exists = (^git branch --list $branch_name | complete | get stdout | str trim | is-not-empty)

  if $local_branch_exists {
    print $"Release branch ($branch_name) already exists locally, checking out"
    ^git checkout $branch_name
  } else {
    # Check if branch exists on remote
    let remote_branch_exists = (^git ls-remote --heads origin $branch_name | complete | get stdout | str trim | is-not-empty)

    if $remote_branch_exists {
      print $"Release branch ($branch_name) exists on remote, checking out and tracking"
      ^git checkout -b $branch_name $"origin/($branch_name)"
    } else {
      print $"Creating new release branch: ($branch_name)"
      ^git checkout -b $branch_name
      ^git push origin $branch_name
    }
  }

  $version
}

# Update Cargo.toml version, return list of files
export def update-cargo-toml [version: string] {
  print $"Updating Cargo.toml version to ($version)"
  let cargo_toml = (open Cargo.toml)
  let updated_cargo = ($cargo_toml | upsert package.version $version)
  $updated_cargo | to toml | save --force Cargo.toml
  ^cargo check
  print "Updated Cargo.toml and Cargo.lock"
  ["Cargo.toml" "Cargo.lock"]
}

# Commit files with message, accept list of files
export def commit-files [message: string] {
  let files = $in
  print $"Committing ($files | str join ', ')"
  ^git add ...$files

  # Check if there are changes to commit
  let has_changes = (^git diff --cached --quiet | complete | get exit_code) != 0

  if $has_changes {
    ^git commit -m $message
    ^git push --force-with-lease origin HEAD
    print "Committed and pushed changes"
  } else {
    print "No changes to commit"
  }
}

# Generate per-arch data files
export def generate-platform-data [
  version: string
  artifacts_path: string
  project_name: string
  archive_ext: string = ".tgz"
  hash_suffix: string = "-nix.sha256"
] {
  print $"Generating platform data files for version ($version)"

  # Create data directory
  mkdir data

  # Process each archive file in artifacts (download-artifact creates subdirectories)  
  let archive_files = (glob $"($artifacts_path)/**/*($archive_ext)")

  for $file in $archive_files {
    let filename = ($file | path basename)
    let platform = ($filename | str replace $"($project_name)-($version)-" "" | str replace $archive_ext "")

    # Find corresponding hash file
    let hash_file = ($file | str replace $archive_ext $hash_suffix)
    let hash = (open $hash_file | str trim)

    let url = $"https://github.com/($env.GITHUB_REPOSITORY)/releases/download/v($version)/($filename)"

    # Create platform JSON file
    let platform_data = {
      url: $url
      hash: $hash
    }

    $platform_data | to json | save $"data/($platform).json"
    print $"Generated data/($platform).json"
  }
}

# Create GitHub release
export def create-github-release [version: string] {
  print $"Creating GitHub release v($version)"
  ^gh release create $"v($version)" --title $"Release v($version)" --target $"release/($version)"
  print $"Created GitHub release v($version)"
}

# Upload artifacts to existing GitHub release (accepts files via pipeline)
export def upload-to-release [version: string] {
  let files = $in
  print $"Uploading ($files | length) artifacts to GitHub release v($version)"
  ^gh release upload $"v($version)" ...$files
  print $"Successfully uploaded artifacts to release v($version)"
}

# Generate platform data and return list of generated files
export def generate-platform-data-for [version: string project_name: string artifacts_path: string = "./artifacts"] {
  let branch_name = $"release/($version)"

  print $"Checking out release branch: ($branch_name)"
  ^git checkout $branch_name

  generate-platform-data $version $artifacts_path $project_name

  # Return the actual files that were created
  glob "data/*.json"
}

# Setup git user for GitHub Actions
export def setup-git-user [] {
  print "Setting up git user for GitHub Actions"
  ^git config --global user.name "github-actions[bot]"
  ^git config --global user.email "github-actions[bot]@users.noreply.github.com"
}

# Merge release branch back to main and cleanup
export def merge-and-cleanup [version: string] {
  let branch_name = $"release/($version)"

  print "Merging release branch to main"
  ^git checkout main
  ^git fetch origin $branch_name
  ^git merge --squash $"origin/($branch_name)"
  ^git commit -m $"Release ($version)"
  ^git push origin main

  print "Cleaning up release branch"
  ^git push origin --delete $branch_name

  print $"Release ($version) completed successfully!"
}
