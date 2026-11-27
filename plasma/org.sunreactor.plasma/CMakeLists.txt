cmake_minimum_required(VERSION 3.22)

project(sunreactor-plasma-panel VERSION 0.14.1 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

find_package(ECM 6.0 REQUIRED NO_MODULE)
list(APPEND CMAKE_MODULE_PATH ${ECM_MODULE_PATH})

include(KDEInstallDirs6)
include(KDECMakeSettings)
include(KDECompilerSettings NO_POLICY_SCOPE)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Qml Quick Network)
# Plasma is only needed to install the package; the plugin, its tests and the
# screenshot harness build without it (e.g. while KDE Frameworks are mid-update).
find_package(Plasma CONFIG QUIET)

qt_policy(SET QTP0001 NEW)

# Plasma 6 package
if (Plasma_FOUND)
    # Keep design briefs and skin brand notes in the source tree only.  The
    # package installer otherwise copies every file below package verbatim.
    install(
        DIRECTORY package/
        DESTINATION ${KDE_INSTALL_DATADIR}/plasma/plasmoids/org.sunreactor.plasma
        PATTERN ".svn" EXCLUDE
        PATTERN "*.qmlc" EXCLUDE
        PATTERN "CMakeLists.txt" EXCLUDE
        PATTERN "Messages.sh" EXCLUDE
        PATTERN "dummydata" EXCLUDE
        PATTERN "*.md" EXCLUDE
        PATTERN "*.MD" EXCLUDE
        PATTERN "*.frag" EXCLUDE
    )
else()
    message(WARNING "Plasma not found: the plasmoid package will not be installed")
endif()

# QML C++ Backend Plugin
add_library(sunreactorplugin SHARED)

target_sources(sunreactorplugin PRIVATE
    plugin/sunreactorclient.cpp
    plugin/sunreactorclient.h
    plugin/sunreactorclient_private.h
    plugin/sunreactorclient_commands.cpp
    plugin/sunreactorclient_queue.cpp
    plugin/sunreactorclient_transport.cpp
    plugin/sunreactorclient_status.cpp
    plugin/sunreactortypes.h
    plugin/themenormalize.cpp
    plugin/themenormalize.h
    plugin/pixelitems.cpp
    plugin/pixelitems.h
    plugin/pixelart.cpp
    plugin/pixelart.h
    plugin/pixelsprite.cpp
    plugin/pixelsprite.h
    plugin/glyphtables.cpp
    plugin/glyphtables.h
    plugin/dotmatrix.cpp
    plugin/dotmatrix.h
)

target_include_directories(sunreactorplugin PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/plugin
)

target_link_libraries(sunreactorplugin
    PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Qml
        Qt6::Quick
        Qt6::Network
)

target_compile_definitions(sunreactorplugin PRIVATE QT_NO_CAST_FROM_ASCII)

qt_add_qml_module(sunreactorplugin
    URI org.sunreactor.plasma
    VERSION 1.0
    OUTPUT_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/qml/org/sunreactor/plasma
)

if (TARGET sunreactorpluginplugin)
    set_target_properties(sunreactorpluginplugin PROPERTIES
        PREFIX "lib"
        INSTALL_RPATH "$ORIGIN;${CMAKE_INSTALL_PREFIX}/lib;${CMAKE_INSTALL_PREFIX}/lib/qt6/qml/org/sunreactor/plasma"
    )
endif()

install(
    TARGETS sunreactorplugin sunreactorpluginplugin
    DESTINATION ${KDE_INSTALL_QMLDIR}/org/sunreactor/plasma
)

install(
    FILES
        ${CMAKE_CURRENT_BINARY_DIR}/qml/org/sunreactor/plasma/qmldir
        ${CMAKE_CURRENT_BINARY_DIR}/qml/org/sunreactor/plasma/sunreactorplugin.qmltypes
    DESTINATION ${KDE_INSTALL_QMLDIR}/org/sunreactor/plasma
)

install(
    FILES
        package/contents/icons/org.sunreactor.plasma.svg
        package/contents/icons/org.sunreactor.plasma-symbolic.svg
    DESTINATION ${KDE_INSTALL_ICONDIR}/hicolor/scalable/apps
)

include(CTest)

# Automated Test Target
if (BUILD_TESTING)
    find_package(Qt6 REQUIRED COMPONENTS Test)
    target_compile_definitions(sunreactorplugin PRIVATE SUNREACTOR_ENABLE_HARNESS)

    # Shader packs (package/contents/ui/skins/*/shaders/*.qsb) are committed, so the
    # widget needs no build tools. With qsb present, `make plasmoid_shaders` rebuilds
    # them from their .frag sources, and the test fails when a committed pack is stale.
    find_program(QSB_EXECUTABLE NAMES qsb
        HINTS "${QT6_INSTALL_PREFIX}/${QT6_INSTALL_BINS}" /usr/lib/qt6/bin)
    if (QSB_EXECUTABLE)
        add_custom_target(plasmoid_shaders
            COMMAND ${CMAKE_COMMAND} -E env QSB=${QSB_EXECUTABLE}
                    ${CMAKE_CURRENT_SOURCE_DIR}/tools/build_shaders.sh
            WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
            COMMENT "Rebuilding the shader packs with qsb"
            VERBATIM)
        add_test(NAME shader_packs_current
            COMMAND ${CMAKE_COMMAND} -E env QSB=${QSB_EXECUTABLE}
                    ${CMAKE_CURRENT_SOURCE_DIR}/tools/build_shaders.sh --check)
    endif()

    add_executable(test_sunreactorclient
        tests/test_sunreactorclient.cpp
        plugin/sunreactorclient.cpp
        plugin/sunreactorclient_commands.cpp
        plugin/sunreactorclient_queue.cpp
        plugin/sunreactorclient_transport.cpp
        plugin/sunreactorclient_status.cpp
        plugin/themenormalize.cpp
    )

    target_compile_definitions(test_sunreactorclient PRIVATE SUNREACTOR_ENABLE_HARNESS)

    target_include_directories(test_sunreactorclient PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/plugin
    )

    target_link_libraries(test_sunreactorclient PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Network
        Qt6::Test
    )

    add_test(NAME test_sunreactorclient COMMAND test_sunreactorclient)
    set_tests_properties(test_sunreactorclient PROPERTIES
        ENVIRONMENT "XDG_RUNTIME_DIR=/nonexistent/sunreactor-test")

    # Dot-matrix primitives: glyph coverage and the pitch snapping rule (no QML).
    add_executable(test_dotmatrix
        tests/test_dotmatrix.cpp
        plugin/dotmatrix.cpp
        plugin/glyphtables.cpp
    )

    target_include_directories(test_dotmatrix PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/plugin
    )

    target_link_libraries(test_dotmatrix PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Qml
        Qt6::Quick
        Qt6::Test
    )

    add_test(NAME test_dotmatrix COMMAND test_dotmatrix)
    set_tests_properties(test_dotmatrix PROPERTIES
        ENVIRONMENT "XDG_RUNTIME_DIR=/nonexistent/sunreactor-test")

    # Pixel glyph layout regression: a daemon-controlled long label must not make
    # elision rescan the whole string once per character.
    add_executable(test_pixelart
        tests/test_pixelart.cpp
        plugin/pixelart.cpp
        plugin/pixelitems.cpp
        plugin/glyphtables.cpp
    )

    target_include_directories(test_pixelart PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/plugin
    )

    target_link_libraries(test_pixelart PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Qml
        Qt6::Quick
        Qt6::Test
    )

    add_test(NAME test_pixelart COMMAND test_pixelart)
    set_tests_properties(test_pixelart PROPERTIES
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_FORCE_STDERR_LOGGING=1;XDG_RUNTIME_DIR=/nonexistent/sunreactor-test")

    # Theme engine tests: QML TestCase against the plugin in the build tree.
    find_program(QMLTESTRUNNER_EXECUTABLE NAMES qmltestrunner6 qmltestrunner
        HINTS "${QT6_INSTALL_PREFIX}/${QT6_INSTALL_BINS}" /usr/lib/qt6/bin)
    if (QMLTESTRUNNER_EXECUTABLE)
        add_test(NAME test_engine
            COMMAND ${QMLTESTRUNNER_EXECUTABLE} -input ${CMAKE_CURRENT_SOURCE_DIR}/tests/tst_engine.qml
                    -import ${CMAKE_CURRENT_BINARY_DIR}/qml)
        set_tests_properties(test_engine PROPERTIES
            ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_FORCE_STDERR_LOGGING=1;QML_IMPORT_PATH=${CMAKE_CURRENT_BINARY_DIR}/qml;XDG_RUNTIME_DIR=/nonexistent/sunreactor-test")

        # Shared data rules: day-curve lookups, display state, the terminal Displays control.
        add_test(NAME test_datarules
            COMMAND ${QMLTESTRUNNER_EXECUTABLE} -input ${CMAKE_CURRENT_SOURCE_DIR}/tests/tst_datarules.qml
                    -import ${CMAKE_CURRENT_BINARY_DIR}/qml)
        set_tests_properties(test_datarules PROPERTIES
            ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_FORCE_STDERR_LOGGING=1;QML_IMPORT_PATH=${CMAKE_CURRENT_BINARY_DIR}/qml;XDG_RUNTIME_DIR=/nonexistent/sunreactor-test")

        # Copy-command field of the not-connected notice.
        add_test(NAME test_copycmd
            COMMAND ${QMLTESTRUNNER_EXECUTABLE} -input ${CMAKE_CURRENT_SOURCE_DIR}/tests/tst_copycmd.qml
                    -import ${CMAKE_CURRENT_BINARY_DIR}/qml)
        set_tests_properties(test_copycmd PROPERTIES
            ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_FORCE_STDERR_LOGGING=1;QML_IMPORT_PATH=${CMAKE_CURRENT_BINARY_DIR}/qml;XDG_RUNTIME_DIR=/nonexistent/sunreactor-test")

        # Theme picker: the button and the popover, clicks and keys (offscreen).
        add_test(NAME test_picker
            COMMAND ${QMLTESTRUNNER_EXECUTABLE} -input ${CMAKE_CURRENT_SOURCE_DIR}/tests/tst_picker.qml
                    -import ${CMAKE_CURRENT_BINARY_DIR}/qml)
        set_tests_properties(test_picker PROPERTIES
            ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_FORCE_STDERR_LOGGING=1;QML_IMPORT_PATH=${CMAKE_CURRENT_BINARY_DIR}/qml;XDG_RUNTIME_DIR=/nonexistent/sunreactor-test")

        # Shared slider commit policy (CommitThrottle).
        add_test(NAME test_commit_throttle
            COMMAND ${QMLTESTRUNNER_EXECUTABLE} -input ${CMAKE_CURRENT_SOURCE_DIR}/tests/tst_commit_throttle.qml
                    -import ${CMAKE_CURRENT_BINARY_DIR}/qml)
        set_tests_properties(test_commit_throttle PROPERTIES
            ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_FORCE_STDERR_LOGGING=1;QML_IMPORT_PATH=${CMAKE_CURRENT_BINARY_DIR}/qml;XDG_RUNTIME_DIR=/nonexistent/sunreactor-test")
    endif()

    add_executable(test_cyberpunk_layout
        tests/test_cyberpunk_layout.cpp
    )

    target_compile_definitions(test_cyberpunk_layout PRIVATE
        QML_IMPORT_DIR="${CMAKE_CURRENT_BINARY_DIR}/qml"
        PLASMA_UI_DIR="${CMAKE_CURRENT_SOURCE_DIR}/package/contents/ui"
    )

    target_link_libraries(test_cyberpunk_layout PRIVATE
        Qt6::Core
        Qt6::Gui
        Qt6::Network
        Qt6::Qml
        Qt6::Quick
        Qt6::Test
    )

    add_test(NAME test_cyberpunk_layout COMMAND test_cyberpunk_layout)
    set_tests_properties(test_cyberpunk_layout PROPERTIES
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_SCALE_FACTOR=1.35;QT_FORCE_STDERR_LOGGING=1;XDG_RUNTIME_DIR=/nonexistent/sunreactor-test")
endif()
