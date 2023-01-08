const path = require('path')
const toml = require('toml')
const fs = require('fs')
const cv = require('compare-versions')
const { spawn } = require('child_process')

/*
  This script verifies that versions in package.json,
  and Cargo.toml are all the same. And if yes, it will set $GITHUB_OUTPUT
  release_version accordingly.

  In addition, if REL_TAG env var is set, this script will set $GITHUB_OUTPUT
  should_release to true if semantic version of release_version is higher
  than REL_TAG, and to false if not.
*/

const latestReleaseTag = process.env.REL_TAG?.replace(/^v/, '')

const packageVersion = require('../client/package.json').version

const cargoConfig = path.join(__dirname, '..', 'server', 'Cargo.toml')

const config = toml.parse(fs.readFileSync(cargoConfig, 'utf-8'))
const tomlVersion = config.package.version

console.log(`
Package.json version  : ${packageVersion}
Cargo.toml version    : ${tomlVersion}`)

const versions = [packageVersion, tomlVersion]
let extra = ''
if (latestReleaseTag) {
  extra = `Latest GH release tag : ${latestReleaseTag}\n`
  versions.push(latestReleaseTag)
}
console.log(extra)

// first validate format
for (const v of versions) {
  if (!cv.validate(v)) {
    console.log('invalid semantic version format found!')
    process.exit(1)
  }
}

// next check if all versions are the same, failing the job if not
if (!cv.compare(packageVersion, tomlVersion, '=')) {
  console.log('different version found in package.json and cargo.toml!')
  process.exit(1)
}

const setGithubOutput = (k, v) => {
  spawn('echo', [`${k}=${v}`, '>>', '$GITHUB_OUTPUT'], {
    shell: true,
    stdio: 'inherit'
  })
}
// output the version
console.log(`setting current_version to ${tomlVersion}`)
setGithubOutput('current_version', tomlVersion)

// determine if release is needed
// defaults to true if latestReleaseTag is undefined since this could be the first one

const needed = latestReleaseTag ? cv.compare(tomlVersion, latestReleaseTag, '>') : true
console.log(`setting should_release to ${needed}`)
setGithubOutput('should_release', needed)
