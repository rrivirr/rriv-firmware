# [1.5.0-beta.4](https://github.com/rrivirr/rriv-firmware/compare/v1.5.0-beta.3...v1.5.0-beta.4) (2026-01-14)


### Bug Fixes

* lorawan flow fixes ([fc43564](https://github.com/rrivirr/rriv-firmware/commit/fc43564d2dfa46933ee0d2e4780640100d7a8c82))

# [1.5.0-beta.3](https://github.com/rrivirr/rriv-firmware/compare/v1.5.0-beta.2...v1.5.0-beta.3) (2026-01-13)


### Bug Fixes

* indexes and messages ([50de01b](https://github.com/rrivirr/rriv-firmware/commit/50de01b6fe78b46a4e20c2981dafb13cdaa26254))

# [1.5.0-beta.2](https://github.com/rrivirr/rriv-firmware/compare/v1.5.0-beta.1...v1.5.0-beta.2) (2026-01-13)


### Bug Fixes

* minor compiler errors ([de55a09](https://github.com/rrivirr/rriv-firmware/commit/de55a09085f04fc40722bfe62218f392484c6110))

# [1.5.0-beta.1](https://github.com/rrivirr/rriv-firmware/compare/v1.4.0...v1.5.0-beta.1) (2026-01-13)


### Bug Fixes

* cleaned up a lot of warnings ([f8be740](https://github.com/rrivirr/rriv-firmware/commit/f8be740aed4e027c29bc13488e8d4ad6c49249b1))
* correct name of firmware elfs ([64057c6](https://github.com/rrivirr/rriv-firmware/commit/64057c67ee05d8c2f39c718cfad4065842f09fc1))
* debug false, cleanup ([10ef797](https://github.com/rrivirr/rriv-firmware/commit/10ef797e995fe296419efbf3d70cc0552f219094))
* duplicated char bug ([57ffb9f](https://github.com/rrivirr/rriv-firmware/commit/57ffb9ff94a9f395a64de8f38f145e4406b2ce47))
* empty bytes no longer necessary ([07e8cf6](https://github.com/rrivirr/rriv-firmware/commit/07e8cf6a3145e84fd277729998fc274b1e50ab8c))
* empty bytes not necessary ([0aabb26](https://github.com/rrivirr/rriv-firmware/commit/0aabb2644f677434d115b90aba29dc4255f7ec4d))
* error handling and formatting ([1eeb343](https://github.com/rrivirr/rriv-firmware/commit/1eeb343d2d46c3b9c408f220d9e16c053a093918))
* handled ID lenght > 6 ([fa7b88e](https://github.com/rrivirr/rriv-firmware/commit/fa7b88e61be67e449160d29f2731d0df3d17f210))
* max value count for modbus telemeter ([330ce88](https://github.com/rrivirr/rriv-firmware/commit/330ce8854ca2d28b1dc092c8b5aa74a62fa9c61f))
* modified I2C addresses ([9ef8c09](https://github.com/rrivirr/rriv-firmware/commit/9ef8c099f20dbaf793b45ae94f5fa6e450195e59))
* only create file when writing ([1d9d287](https://github.com/rrivirr/rriv-firmware/commit/1d9d287bb0192bbf33fc9a2b24f4a078375cd0f3))
* only create file when writing ([c3df727](https://github.com/rrivirr/rriv-firmware/commit/c3df727347176f978c96ebb315dabfbb57ca65ca))
* RAK3172 handle initial messages that can fill buffers freezing usart comms ([faff203](https://github.com/rrivirr/rriv-firmware/commit/faff203042a0ba12fe4d2eca4144e45e0a9d2a39))
* reducing firmware size, cleaning up ([9d599d2](https://github.com/rrivirr/rriv-firmware/commit/9d599d2f58b4573c9b279c8252ace2da1970ade5))
* removed the use of format! from out codes ([a88982f](https://github.com/rrivirr/rriv-firmware/commit/a88982f1667022bfcb9a84af424014cd96c7cc36))
* restore millis ([4506480](https://github.com/rrivirr/rriv-firmware/commit/4506480c81d2c6251e19bd190b9befcacef06d18))
* run build automation with ref ([a3cbac0](https://github.com/rrivirr/rriv-firmware/commit/a3cbac0a4d7dcf7a87eb3d0e5df1446c664d2f4f))
* some cleanup ([cdb9f72](https://github.com/rrivirr/rriv-firmware/commit/cdb9f723dcf7262c149f9210bc75be737f74ee57))


### Features

* adc port 5 as one wire working ([f710513](https://github.com/rrivirr/rriv-firmware/commit/f71051345ecf9f3345a42ee4051d74fd46b548c8))
* initial integration of atlas ec sensor ([7999799](https://github.com/rrivirr/rriv-firmware/commit/799979923aaba25ec1cbc6684e8a5c50efdc6ba6))
* initial_state with on and off strings [#94](https://github.com/rrivirr/rriv-firmware/issues/94) ([a8ed638](https://github.com/rrivirr/rriv-firmware/commit/a8ed638b03920e9150292dc2aacf6174179aa467))
* merged beta into main ([183ef6f](https://github.com/rrivirr/rriv-firmware/commit/183ef6f99d4c1a76ab7aca123bc561fa5ed47985))
* merged in defmt ([cfdae12](https://github.com/rrivirr/rriv-firmware/commit/cfdae12a53a32f6956dbd6296967073597b4af52))
* modbus driver ([8f5970a](https://github.com/rrivirr/rriv-firmware/commit/8f5970a83380d293c29c32d1d03c76a296cd114b))
* rs485 communication minimally working ([cec6f5f](https://github.com/rrivirr/rriv-firmware/commit/cec6f5fe12b51779444d44010744048d9e059d36))
* secondary UART ([695b29d](https://github.com/rrivirr/rriv-firmware/commit/695b29d6175a10db9f05e210b5aaf975ded9d5d5))
* send varialbe number of driver measurements over lorawan ([512a437](https://github.com/rrivirr/rriv-firmware/commit/512a43777dd4fe1bad81092a7ab76e4b7355251d))
* set up and fixes to groundwater_rtu driver ([7f5b4db](https://github.com/rrivirr/rriv-firmware/commit/7f5b4db800a25b82736801ba6bd3de5419e74982))
* setup processing for modbus rtu ([02de62c](https://github.com/rrivirr/rriv-firmware/commit/02de62c3470244d2192629f76373a4a510239dac))
* support to set sensor settings individually ([f03c712](https://github.com/rrivirr/rriv-firmware/commit/f03c7125e732ac8c060b1fdbf81bd133f6cdaf02))
* switch to defmt ([56b7b43](https://github.com/rrivirr/rriv-firmware/commit/56b7b43918f5aa2d25b8a45f10745ead0c3f1cc1))

# [1.4.0](https://github.com/rrivirr/rriv-firmware/compare/v1.3.0...v1.4.0) (2026-01-13)


### Bug Fixes

* enable both single and mux ring temp drivers ([9780c79](https://github.com/rrivirr/rriv-firmware/commit/9780c79439db11d28d456e313c155ead943acc7a))


### Features

* basic mux test working ([14b6b78](https://github.com/rrivirr/rriv-firmware/commit/14b6b78f5b91984b01e509638ff902e7e346dfa7))
* configurable number of sensors ([ef86451](https://github.com/rrivirr/rriv-firmware/commit/ef86451c5d9c02a08131cbb80b234a7dfb298b75))

# [1.3.0](https://github.com/rrivirr/rriv-firmware/compare/v1.2.0...v1.3.0) (2026-01-13)


### Features

* adc_internal temperature driver ([89778db](https://github.com/rrivirr/rriv-firmware/commit/89778db4c0759f713016ec38fa9500f67624cff4))

# [1.2.0](https://github.com/rrivirr/rriv-firmware/compare/v1.1.12...v1.2.0) (2025-12-08)


### Bug Fixes

* period and ratio parsing ([49c3751](https://github.com/rrivirr/rriv-firmware/commit/49c3751dfac14739f6ef59fe2cb65cf57b016470))


### Features

* duty_cycle implementation for timed_switch ([4adf22e](https://github.com/rrivirr/rriv-firmware/commit/4adf22e2b7c6c73d35cd750eb5a63774f8216619))

## [1.1.12](https://github.com/rrivirr/rriv-firmware/compare/v1.1.11...v1.1.12) (2025-12-03)


### Bug Fixes

* run build automation with ref ([5693c82](https://github.com/rrivirr/rriv-firmware/commit/5693c82297daf72d62c82debd698cade1a725909))

## [1.1.11](https://github.com/rrivirr/rriv-firmware/compare/v1.1.10...v1.1.11) (2025-11-12)


### Bug Fixes

* enable setting of interactive_logging ([49ebd40](https://github.com/rrivirr/rriv-firmware/commit/49ebd407d461529751b462a0e0afa33664d94d58))

## [1.1.10](https://github.com/rrivirr/rriv-firmware/compare/v1.1.9...v1.1.10) (2025-10-18)


### Bug Fixes

* correct path ([6b6f4ed](https://github.com/rrivirr/rriv-firmware/commit/6b6f4ed3be8a0219c69d2881ad121979f4e31087))

## [1.1.9](https://github.com/rrivirr/rriv-firmware/compare/v1.1.8...v1.1.9) (2025-10-18)


### Bug Fixes

* correct syntax for debug build ([f0c4595](https://github.com/rrivirr/rriv-firmware/commit/f0c4595bad6bd3839393363975ff8d242881b71b))

## [1.1.8](https://github.com/rrivirr/rriv-firmware/compare/v1.1.7...v1.1.8) (2025-10-18)


### Bug Fixes

* also build debug build ([a99ed6d](https://github.com/rrivirr/rriv-firmware/commit/a99ed6d19b2911664732b79e9ecb4a18419b3169))

## [1.1.7](https://github.com/rrivirr/rriv-firmware/compare/v1.1.6...v1.1.7) (2025-10-15)


### Bug Fixes

* force release ([eb188d3](https://github.com/rrivirr/rriv-firmware/commit/eb188d35a9fcbb54c9e9565472ef54397fcff26b))

## [1.1.6](https://github.com/rrivirr/rriv-firmware/compare/v1.1.5...v1.1.6) (2025-10-15)


### Bug Fixes

* force release ([fcb1c22](https://github.com/rrivirr/rriv-firmware/commit/fcb1c22ae65afe89758d41a4c2100f0f49842c10))

## [1.1.5](https://github.com/rrivirr/rriv-firmware/compare/v1.1.4...v1.1.5) (2025-10-15)


### Bug Fixes

* Update semantic-release.yaml ([6cd03e8](https://github.com/rrivirr/rriv-firmware/commit/6cd03e87002c0ab8b2d166d52970860a735f60f3))

## [1.1.4](https://github.com/rrivirr/rriv-firmware/compare/v1.1.3...v1.1.4) (2025-10-15)


### Bug Fixes

* some cargo fixes ([f4dabef](https://github.com/rrivirr/rriv-firmware/commit/f4dabefef751d4754be263217d7b057c1321586a))

## [1.1.3](https://github.com/rrivirr/rriv-firmware/compare/v1.1.2...v1.1.3) (2025-10-14)


### Bug Fixes

* adjust setup order so that rtc clock init errors are caught by the watchdog ([6a39f26](https://github.com/rrivirr/rriv-firmware/commit/6a39f262f589c3698a243e94e194d9f4d75e73aa))

## [1.1.2](https://github.com/rrivirr/rriv-firmware/compare/v1.1.1...v1.1.2) (2025-10-14)


### Bug Fixes

* added missing fn in ring_temp.rs ([faacce4](https://github.com/rrivirr/rriv-firmware/commit/faacce48d3a2f2b59ac5f7931a1a4dc2bec23589))
* force release ([c34f46b](https://github.com/rrivirr/rriv-firmware/commit/c34f46b7e05cea614998f2b267261ab5be4c285f))
* include missing implementations ([9f5cbba](https://github.com/rrivirr/rriv-firmware/commit/9f5cbbaa2e7d800b8c3957945759fc4e6dadcaa0))
* modified I2C addresses ([2fefb06](https://github.com/rrivirr/rriv-firmware/commit/2fefb06375216fd7047ae9b2e4d4aacdc853b3f8))

## [1.1.1](https://github.com/rrivirr/rriv-firmware/compare/v1.1.0...v1.1.1) (2025-09-13)


### Bug Fixes

* always call the firmware as firmware.bin ([9e01213](https://github.com/rrivirr/rriv-firmware/commit/9e012138bfe6d35facd0d083c5aea85b23814c8b))
* CD outputs ([c2b14b8](https://github.com/rrivirr/rriv-firmware/commit/c2b14b88c442c7a87dc59a619e7b70050763b43f))
* get firmware version from tag ([21ddb3c](https://github.com/rrivirr/rriv-firmware/commit/21ddb3cd38b0ba88ca208225c0ed8d503946a9ab))
* test CD ([52556cd](https://github.com/rrivirr/rriv-firmware/commit/52556cdba01d334c9de2cd9f200066c7acf238a5))
* test CD ([dea9fae](https://github.com/rrivirr/rriv-firmware/commit/dea9faeb338ac54ca8e8f085859e98c9fed78410))
* test CD ([a5a7113](https://github.com/rrivirr/rriv-firmware/commit/a5a7113fac2614bfaef0a782f0efa90702ba3c30))
* test CD ([1193540](https://github.com/rrivirr/rriv-firmware/commit/11935402e4e44c2c4f4aa2a63dc280bf4b9c0fae))
* test CD ([67bd076](https://github.com/rrivirr/rriv-firmware/commit/67bd0769a519264450753dedd31975973180031e))
* test CD ([f695ec5](https://github.com/rrivirr/rriv-firmware/commit/f695ec5c9d5860f6652198dc591c7273af0ef088))
* test CD ([31557ca](https://github.com/rrivirr/rriv-firmware/commit/31557cae2b4e0bbc0921e5b27da6c3359ada2955))
* test CD ([a39f5b4](https://github.com/rrivirr/rriv-firmware/commit/a39f5b435ff836e421f0a82ae84219bd2533bf21))
* test the beta pre-release ([93115a3](https://github.com/rrivirr/rriv-firmware/commit/93115a3d5b1867ed359b8fe325b06e18f42401c7))

## [1.1.1-beta.3](https://github.com/rrivirr/rriv-firmware/compare/v1.1.1-beta.2...v1.1.1-beta.3) (2025-09-12)


### Bug Fixes

* get firmware version from tag ([21ddb3c](https://github.com/rrivirr/rriv-firmware/commit/21ddb3cd38b0ba88ca208225c0ed8d503946a9ab))
* test CD ([52556cd](https://github.com/rrivirr/rriv-firmware/commit/52556cdba01d334c9de2cd9f200066c7acf238a5))

## [1.1.1-beta.2](https://github.com/rrivirr/rriv-firmware/compare/v1.1.1-beta.1...v1.1.1-beta.2) (2025-09-12)


### Bug Fixes

* test CD ([dea9fae](https://github.com/rrivirr/rriv-firmware/commit/dea9faeb338ac54ca8e8f085859e98c9fed78410))

## [1.1.1-beta.1](https://github.com/rrivirr/rriv-firmware/compare/v1.1.0...v1.1.1-beta.1) (2025-09-12)


### Bug Fixes

* CD outputs ([c2b14b8](https://github.com/rrivirr/rriv-firmware/commit/c2b14b88c442c7a87dc59a619e7b70050763b43f))
* test CD ([a5a7113](https://github.com/rrivirr/rriv-firmware/commit/a5a7113fac2614bfaef0a782f0efa90702ba3c30))
* test CD ([1193540](https://github.com/rrivirr/rriv-firmware/commit/11935402e4e44c2c4f4aa2a63dc280bf4b9c0fae))
* test CD ([67bd076](https://github.com/rrivirr/rriv-firmware/commit/67bd0769a519264450753dedd31975973180031e))
* test CD ([f695ec5](https://github.com/rrivirr/rriv-firmware/commit/f695ec5c9d5860f6652198dc591c7273af0ef088))
* test CD ([31557ca](https://github.com/rrivirr/rriv-firmware/commit/31557cae2b4e0bbc0921e5b27da6c3359ada2955))
* test CD ([a39f5b4](https://github.com/rrivirr/rriv-firmware/commit/a39f5b435ff836e421f0a82ae84219bd2533bf21))
* test the beta pre-release ([93115a3](https://github.com/rrivirr/rriv-firmware/commit/93115a3d5b1867ed359b8fe325b06e18f42401c7))

# [1.1.0](https://github.com/rrivirr/rriv-firmware/compare/v1.0.3...v1.1.0) (2025-09-12)


### Features

* initial state for timed switch ([ab7b4e0](https://github.com/rrivirr/rriv-firmware/commit/ab7b4e00170e583e2d8e36a53ca0c368a7e2578c))

## [1.0.3](https://github.com/rrivirr/rriv-rust/compare/v1.0.2...v1.0.3) (2025-09-11)


### Bug Fixes

* test CD ([f8b1b99](https://github.com/rrivirr/rriv-rust/commit/f8b1b99aa8546a874f59ac845cd30a352c99060a))

## [1.0.2](https://github.com/rrivirr/rriv-rust/compare/v1.0.1...v1.0.2) (2025-09-11)


### Bug Fixes

* test CD ([3c54890](https://github.com/rrivirr/rriv-rust/commit/3c5489071c464d59564bbce49fd9cc80cd25cedc))

## [1.0.1](https://github.com/rrivirr/rriv-rust/compare/v1.0.0...v1.0.1) (2025-09-10)


### Bug Fixes

* trigger a release ([65f2486](https://github.com/rrivirr/rriv-rust/commit/65f2486243fc51a1cef4a2ee70f9131e584e1d1c))
* trigger a release ([11f7525](https://github.com/rrivirr/rriv-rust/commit/11f7525490e947f68514b272572729732b538a33))
