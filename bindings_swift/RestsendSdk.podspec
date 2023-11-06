Pod::Spec.new do |s|
    s.name        = "RestsendSdk"
    s.version     = "0.0.1"
    s.summary     = "Restsend client sdk for iOS"
    s.homepage    = "https://github.com/restsend/restsend-rs"
    s.license     = { :type => "MIT" }
    s.authors = { "restsend" => "admin@restsend.com"}
    s.requires_arc = true
    s.swift_version = "5.0"
    s.source   = { :git => "https://github.com/restsend/restsend-rs.git", :tag => s.version }
    s.source_files = "*.{h,swift,modulemap}"
    s.ios.deployment_target = '11.0'
    s.vendored_frameworks = "restsendFFI.xcframework"
end