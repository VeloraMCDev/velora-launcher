# Source and dependency notices

New Velora gateway source follows the existing rights restrictions in the root
[LICENSE.md](../../LICENSE.md). Public source visibility is not a blanket license
grant or permission to use Velora artwork/trademarks.

The plugin JAR contains only this component's compiled classes and generated
Velocity plugin metadata. Runtime API dependencies are supplied by Velocity.
Velocity is licensed under LGPL-3.0; Gson and Guice under Apache-2.0; Adventure
and SLF4J under MIT. Their upstream notices/licenses continue to apply to their
own distributions. Tests additionally use JUnit (EPL-2.0) and Mockito (MIT),
which are not distributed inside the plugin JAR. Gradle wrapper provenance is
retained in the parent integration build. Consult each artifact's accompanying
license for transitive dependencies before redistributing a runtime bundle.
