+++
title = "Iterative Package Development"
description = "Configure the Supervisor for faster package development"

[menu]
  [menu.habitat]
    title = "Iterative Package Development"
    identifier = "habitat/packages/iterative-development"
    parent = "habitat/packages"
    weight = 20

+++

To assist in creating new packages, or modifying existing ones, the Supervisor has an option to allow you to use the configuration directly from a specific directory, rather than the one it includes in the compiled artifact. This can significantly shorten the cycle time when working on configuration and application lifecycle hooks.

Build the plan as you normally would. When you start the Supervisor, pass the name of the directory with your plan inside it:

```bash
$ hab sup run core/redis --config-from /src
```

The Supervisor will now take its configuration and hooks from /src, rather than from the package you previously built. When the configuration is as you want it, do a final rebuild of the package.
