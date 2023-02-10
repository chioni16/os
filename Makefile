asm_src_dir := src/asm
asm_src_files := $(wildcard ${asm_src_dir}/*.asm)
bin_dir := target/bin
asm_obj_files := $(patsubst ${asm_src_dir}/%.asm, ${bin_dir}/%.o, ${asm_src_files})
rust_bin := target/x86_64-unknown-none/debug/libos.a
conf_dir := src/config
iso_dir := target/isofiles
kernel := ${bin_dir}/kernel

.PHONY: all
all: ${bin_dir}/os.iso

${bin_dir}/os.iso: ${kernel}
	mkdir -p ${iso_dir}/boot/grub
	cp ${conf_dir}/grub.cfg ${iso_dir}/boot/grub
	cp ${kernel} ${iso_dir}/boot/
	grub-mkrescue -o ${bin_dir}/os.iso ${iso_dir}
	rm -rf ${iso_dir}

${kernel}: ${asm_obj_files} ${rust_bin}
	ld -n -o ${kernel} -T ${conf_dir}/linker.ld ${asm_obj_files} ${rust_bin}

.PHONY: ${rust_bin}
${rust_bin}: 
	cargo b

${bin_dir}/%.o: ${asm_src_dir}/%.asm
	@mkdir -p ${bin_dir}
	nasm -f elf64 $< -o $@

.PHONY: clean
clean:
	rm -r ${bin_dir}/*
	