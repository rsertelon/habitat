+++
title = "Create an Origin on Builder"
description = "Create an Origin on Builder"

[menu]
  [menu.habitat]
    title = "Create an Origin on Builder"
    identifier = "habitat/builder/builder-origin"
    parent = "habitat/builder"
    weight = 20

+++

Origins are unique namespaces that can be used to denote a particular upstream of a package. For example, the "core" origin is the set of foundational packages that are managed and versioned by the core Chef Habitat maintainers.

From the My Origins page in the Chef Habitat Builder web app, click the **Create origin** button.

> **Note** To join an existing origin, a current member of that origin will need to invite you. Pending invites will appear on the **My Origins** page for you to accept.

<img src="/images/screenshots/create-origin.png">

## Choose an Origin Name

Pick an origin that is your company name, team name, personal name, or some other unique name that you want to associate with a given set of packages. It's important to note that once you have uploaded a package into the depot, the origin that you chose when building that package can neither be edited nor deleted.

## Choose a Privacy Setting

This is the default privacy setting applied to new packages. You can override this setting on individual packages when uploading or connecting a plan file.

Public packages will appear in public search results and can be used by any user, while private packages are restricted to members of the origin.
