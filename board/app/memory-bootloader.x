/* App layout when installed behind the RRIV bootloader (feature `with-bootloader`):
   the bootloader owns 0x08000000-0x0800BFFF (rriv_boot::BOOTLOADER_SIZE) and the
   image header the bootloader checks takes 0x0800C000-0x0800C1FF
   (rriv_boot::APP_VECTORS), so the app's vector table starts at 0x0800C200. */
MEMORY
{
  /* NOTE K = KiBi = 1024 bytes */
  FLASH : ORIGIN = 0x0800C200, LENGTH = 474624
  /* The last 64 bytes of RAM hold the start-up log (rriv_boot::startup). */
  RAM : ORIGIN = 0x20000000, LENGTH = 64K - 64
}

/* This is where the call stack will be allocated. */
/* The stack is of the full descending type. */
/* NOTE Do NOT modify `_stack_start` unless you know what you are doing */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);
