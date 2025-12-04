# 用KeePassXC 的kdbx 文件(单个)做数据库，实现的论坛软件

# how to run

```
cargo run -- \                                               
  -d your-forum.kdbx \
  -P 'yourp4ssword' \
  --listen 127.0.0.1:3000

```


# 请参照 这个来生成 .kdbx 文件
https://github.com/cleanyong/cmd-call-kdbx

**记住，自己先用KeePassXC打开来创建栏目**
