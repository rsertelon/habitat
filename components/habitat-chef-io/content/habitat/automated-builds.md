+++
title = "Automated Builds"
description = "Set up Automated Builds"

[menu]
  [menu.habitat]
    title = "Automated Builds"
    identifier = "habitat/builder/automated-builds Automated Builds"
    parent = "habitat/builder"
    weight = 20

+++

By connecting a plan file in <a href="https://bldr.habitat.sh/#/sign-in" class="link-external" target="_blank">Chef Habitat Builder</a>, you can trigger both manual (via the web UI, or via the `hab` command line) as well as automated package rebuilds whenever a change is merged into the `master` branch of the repository containing your Chef Habitat plan, or when a dependent package updates (rebuilds).

## Connect a Plan

To connect a plan to Builder, view one of your origins (while signed in), click the **Connect a plan file** button, and complete the following steps:

  - Install the Builder GitHub App
  - Choose the GitHub organization and repository containing your Chef Habitat plan
  - Choose a privacy setting for the package
  - Specify container-registry publishing settings (optional)
  - Specify auto-build option (default is off)

### Auto-build Option

The auto-build option controls whether or not your package will get automatically re-built. This option is a useful capability to have - for example, if you have a demo app that doesn't need to be kept constantly up to date when some underlying dependency updates. Auto-build encompasses both builds that are triggered by Github web hooks (on commits to master), as well as builds that are triggered by a dependency updating.

By default, new plan connections will have auto-build turned off.
