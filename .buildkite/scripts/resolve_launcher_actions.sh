#!/bin/bash

# If the user informed us that we need to build a new set of Launcher
# artifacts, we add steps to do so into the pipeline. Otherwise, we
# determin what the currently stable Launcher artifacts are and
# promote them into our release channel

set -euo pipefail
source .buildkite/scripts/shared.sh

# Take the latest release of a given package in a specific channel in
# Builder and promote it into another channel.
promote_from_one_channel_to_another() {
    target="${1}"       # e.g. "x86_64-linux"
    package_name="${2}" # e.g. "hab-launcher"
    from_channel="${3}" # e.g. "stable"
    to_channel="${4}"   # e.g. "rc-0.75.0"

    artifact="$(latest_from_builder "${target}" "${from_channel}" "${package_name}")"
    echo "--- Promoting ${artifact} (${target}) to ${to_channel}"
    # TODO: after 0.79.0 we can reenable this. We are explicitly using curl to upload
    # due to this bug: https://github.com/habitat-sh/builder/issues/940
    # hab pkg promote --auth="${HAB_AUTH_TOKEN}" "${artifact}" "${to_channel}"

    # Create the channel, if necessary.
    #
    # Don't use --fail here, because trying to create a channel that
    # already exists returns a 409, and we don't want to fail in that case.
    curl --request POST \
         --header "Authorization: Bearer $HAB_AUTH_TOKEN" \
         --verbose \
         "https://bldr.habitat.sh/v1/depot/channels/core/${to_channel}"

    # Extract the individual bits of the fully-qualified identifier we
    # just retrieved.
    IFS="/" read -r _ _ version release <<< "${artifact}"

    # Promote the uploaded package into the channel.
    curl --request PUT \
         --header "Authorization: Bearer $HAB_AUTH_TOKEN" \
         --fail \
         --verbose \
         "https://bldr.habitat.sh/v1/depot/channels/core/${to_channel}/pkgs/${package_name}/${version}/${release}/promote?&target=${target}"
}

launcher_action=$(buildkite-agent meta-data get "launcher-action");
case "${launcher_action}" in
    "build-new-launcher")
        echo "--- :pipeline: Dynamically adding Launcher build steps to the pipeline"
        buildkite-agent pipeline upload .buildkite/launcher_build_steps.yaml
        ;;
    "use-stable-launcher")
        release_channel=$(get_release_channel)
        echo "--- Adding stable Launcher artifacts to channel '${release_channel}'"
        # We don't build the launcher on macOS
        launcher_platforms=("x86_64-linux"
                            "x86_64-linux-kernel2"
                            "x86_64-windows")

        for target in "${launcher_platforms[@]}"; do
            promote_from_one_channel_to_another "${target}" "hab-launcher" "stable" "${release_channel}"
        done
        # Don't forget about the Windows Service!
        promote_from_one_channel_to_another "x86_64-windows" "windows-service" "stable" "${release_channel}"
        ;;
    *)
        echo "--- :scream: Unexpected launcher action '${launcher_action}'! ABORT!"
        exit 1
esac
