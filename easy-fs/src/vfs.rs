use crate::efs::DataBlock;

use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
// use spin::{Mutex, MutexGuard};
use up::UPIntrFreeCell;

pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<UPIntrFreeCell<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// We should not acquire efs lock here.
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<UPIntrFreeCell<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .exclusive_access()
            .read(self.block_offset, f)
    }

    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .exclusive_access()
            .modify(self.block_offset, f)
    }

    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_number() as u32);
            }
        }
        None
    }

    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                // info!("BEFORE");
                let fs = self.fs.exclusive_access();
                // info!("AFTER");
                let (block_id, block_offset) =
                    EasyFileSystem::get_disk_inode_pos(fs.inode_area_start_block, inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

    fn increase_size(&self, new_size: u32, disk_inode: &mut DiskInode, fs_alloc_data: u32) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs_alloc_data);
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let op = |root_inode: &mut DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.modify_disk_inode(op).is_some() {
            return None;
        }

        let fs = self.fs.exclusive_access();
        let inode_bitmap = fs.inode_bitmap;
        let data_bitmap = fs.data_bitmap;
        let block_device = fs.block_device.clone();
        let inode_area_start_block = fs.inode_area_start_block;
        drop(fs);

        // alloc a inode with an indirect block
        let new_inode_id = inode_bitmap.alloc(&block_device).unwrap() as u32;
        let (new_inode_block_id, new_inode_block_offset) =
            EasyFileSystem::get_disk_inode_pos(inode_area_start_block, new_inode_id);
        let (block_id, block_offset) =
            EasyFileSystem::get_disk_inode_pos(inode_area_start_block, new_inode_id);

        // may need schedule!
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .exclusive_access()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(
                new_size as u32,
                root_inode,
                data_bitmap.alloc(&block_device).unwrap() as u32,
            );
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        block_cache_sync_all();

        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
    }

    pub fn ls(&self) -> Vec<String> {
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }

    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let size = self.modify_disk_inode(|disk_inode| {
            let mut fs = self.fs.exclusive_access();
            let fs_alloc_data = fs.alloc_data();
            drop(fs);

            // may need schedule!
            self.increase_size((offset + buf.len()) as u32, disk_inode, fs_alloc_data);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }

    pub fn clear(&self) {
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                let fs = self.fs.exclusive_access();
                let block_device = fs.block_device.clone();
                let data_bitmap = fs.data_bitmap;
                let data_area_start_block = fs.data_area_start_block;
                drop(fs);

                // may need schedule!
                get_block_cache(data_block as usize, Arc::clone(&self.block_device))
                    .exclusive_access()
                    .modify(0, |data_block: &mut DataBlock| {
                        data_block.iter_mut().for_each(|p| {
                            *p = 0;
                        })
                    });
                data_bitmap.dealloc(&block_device, (data_block - data_area_start_block) as usize)
            }
        });
        block_cache_sync_all();
    }
}
