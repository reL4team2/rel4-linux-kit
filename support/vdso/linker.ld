SECTIONS
{
  . = 0x1000;
  .text : ALIGN(4K) {
    *(.text)
  }
  .data : ALIGN(4K) {
    *(.data)
  }
}
