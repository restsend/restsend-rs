#/bin/sh
# see also https://github.com/human-solutions/xcframework

CONF=release
DEV=0

while test $# -gt 0
do
    case "$1" in
        --debug) CONF=debug
            ;;
        --dev) DEV=1
            ;;
    esac
    shift
done

# get current script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
VERSION=$(cat ${SCRIPT_DIR}/../bindings_swift/RestsendSdk.podspec|grep s.version | cut -d '"' -f 2|head -1)

BUILD_DIR=.xcbuild
NAME=restsendFFI
DIST_ROOT=bindings_swift

rm -Rf ${DIST_ROOT}/${NAME}.xcframework*
rm -Rf ${BUILD_DIR}
mkdir -p ${BUILD_DIR}/Headers
mkdir -p ${BUILD_DIR}/Modules

echo "▸ Sync sources"
cp bindings_swift/module.h ${BUILD_DIR}/Headers/module.h

echo "▸ Create xcframework ${CONF}"

if [ $DEV -eq 0 ]; then
    WITH_X86="-library ./target/x86_64-apple-ios/${CONF}/librestsend_sdk.a"
fi

xcodebuild -create-xcframework \
-library ./target/aarch64-apple-ios-sim/${CONF}/librestsend_sdk.a \
-library ./target/aarch64-apple-ios/${CONF}/librestsend_sdk.a \
-library ./target/x86_64-apple-darwin/${CONF}/librestsend_sdk.a \
-headers ${BUILD_DIR}/Headers \
-output ${DIST_ROOT}/${NAME}.xcframework


rm -Rf ${BUILD_DIR}

# strip DIST_ROOT from path
if [ $DEV -eq 0 ]; then
    cd ${DIST_ROOT}
    echo "▸ Compress xcframework ${NAME}-${VERSION}.xcframework.zip"
    rm -f /tmp/${NAME}-${VERSION}.xcframework.zip
    zip -x "*.zip" -r /tmp/${NAME}-${VERSION}.xcframework.zip *
    zipsize=`du -m /tmp/${NAME}-${VERSION}.xcframework.zip | cut -f1`
    echo "▸ Compressed xcframework size: ${zipsize}M"
    scp /tmp/${NAME}-${VERSION}.xcframework.zip ubuntu@chat.rddoc.cn:/var/www/chat/downloads/
fi
echo "🎉 Done with config: ${CONF}"
