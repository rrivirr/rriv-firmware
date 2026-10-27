/* The bootloader owns the first 48 KiB of flash (rriv_boot::BOOTLOADER_SIZE). */
MEMORY
{
  FLASH : ORIGIN = 0x08000000, LENGTH = 48K
  /* The last 64 bytes of RAM hold the start-up log (rriv_boot::startup). */
  RAM : ORIGIN = 0x20000000, LENGTH = 64K - 64
}

_stack_start = ORIGIN(RAM) + LENGTH(RAM);
