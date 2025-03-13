
= for_auto_test_folder

.sy  官方源代码  
00in    功能样例
00performance 性能样例

00official_out 官方输出(包括 功能样式性能样例的输出)
04ours_out

1s 我们编译器生成的汇编代码
    |from
  assembler 
    |to 
2o 输出的中间文件  .o 作为后缀  
    |from
  linker
    |to
3elf 生成的可执行文件  .elf 作为后缀
，类似于windows 下的exe

= git 上传代码

