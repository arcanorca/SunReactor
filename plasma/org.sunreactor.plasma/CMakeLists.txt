cmake_minimum_required(VERSION 3.22)

project(sunreactor-plasma-panel VERSION 1.0.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

find_package(ECM 6.0 REQUIRED NO_MODULE)
list(APPEND CMAKE_MODULE_PATH ${ECM_MODULE_PATH})

include(KDEInstallDirs6)
include(KDECMakeSettings)
include(KDECompilerSettings NO_POLICY_SCOPE)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Qml Quick Network Test)
# Plasma is only needed to install the package; the plugin, its tests and the
# screenshot harness build without it (e.g. while KDE Frameworks are mid-update).
find_package(Plasma CONFIG QUIET)

qt_policy(SET QTP0001 NEW)

# Plasma 6 package
if (Plasma_FOUND)
    plasma_install_package(package org.sunreactor.plasma)
else()
    message(WARNING "Plasma not found: the plasmoid package will not be installed")
endif()

# Bundled fonts (licences: package/contents/fonts/FONTS-LICENSES.txt)
install(
    DIRECTORY
        package/contents/fonts
    DESTINATION ${KDE_INSTALL_DATADIR}/plasma/plasmoids/org.sunreactor.plasma/contents
)

# QML C++ Backend Plugin
add_library(sunreactorplugin SHARED)

target_sources(sunreactorplugin PRIVATE
    plugin/sunreactorclient.cpp
    plugin/sunreactorclient.h
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
    TARGETS sunreactorplugin
    DESTINATION ${KDE_INSTALL_LIBDIR}
)

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

# Automated Test Target
enable_testing()

add_executable(test_sunreactorclient
    tests/test_sunreactorclient.cpp
    plugin/sunreactorclient.cpp
    plugin/themenormalize.cpp
)

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

# Theme engine tests: QML TestCase against the plugin in the build tree.
find_program(QMLTESTRUNNER_EXECUTABLE NAMES qmltestrunner6 qmltestrunner
    HINTS "${QT6_INSTALL_PREFIX}/${QT6_INSTALL_BINS}" /usr/lib/qt6/bin)
if (QMLTESTRUNNER_EXECUTABLE)
    add_test(NAME test_engine
        COMMAND ${QMLTESTRUNNER_EXECUTABLE} -input ${CMAKE_CURRENT_SOURCE_DIR}/tests/tst_engine.qml
                -import ${CMAKE_CURRENT_BINARY_DIR}/qml)
    set_tests_properties(test_engine PROPERTIES
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_FORCE_STDERR_LOGGING=1;QML_IMPORT_PATH=${CMAKE_CURRENT_BINARY_DIR}/qml")

    # Theme picker: the button and the popover, clicks and keys (offscreen).
    add_test(NAME test_picker
        COMMAND ${QMLTESTRUNNER_EXECUTABLE} -input ${CMAKE_CURRENT_SOURCE_DIR}/tests/tst_picker.qml
                -import ${CMAKE_CURRENT_BINARY_DIR}/qml)
    set_tests_properties(test_picker PROPERTIES
        ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_FORCE_STDERR_LOGGING=1;QML_IMPORT_PATH=${CMAKE_CURRENT_BINARY_DIR}/qml")
endif()
