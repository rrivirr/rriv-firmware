
wget https://github.com/rrivirr/rriv-firmware/releases/latest/download/rriv-firmware-debug.elf -O ./rriv-firmware-debug.elf
probe-rs download  ./rriv-firmware-debug.elf \
	--chip STM32F103RE  \
 	--protocol swd \
	--allow-erase-all \
	--chip-erase
rm rriv-firmware-debug.elf
