import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:e2xf/event.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'src/rust/api/bridge.dart' as lib;
import 'package:shared_preferences/shared_preferences.dart';
import 'package:macos_secure_bookmarks/macos_secure_bookmarks.dart';

class MainViewModel {
  static const String cfgKey = 'defaultCfg';
  static const String excelPathKey = 'excelPath';
  static const String xmlFolderPathKey = 'xmlFolderPath';
  static const String excelBookmarkKey = 'excelBookmark';
  static const String xmlFolderBookmarkKey = 'xmlFolderBookmark';

  final _secureBookmarks = SecureBookmarks();

  // 存储已解析的 security-scoped 资源，以便 dispose 时释放
  FileSystemEntity? _resolvedExcelFile;
  FileSystemEntity? _resolvedXmlFolder;

  MainViewModel() {
    updateLog("Application initialized.");
  }

  final Event<String> _selectedExcelPath = Event("");
  final Event<String> _selectedXmlFolderPath = Event("");
  String _defaultCfg = "";
  final Event<String> _log = Event("");
  String _cfgErrTip = "";
  final Event<bool> _isLoading = Event(false);

  final ScrollController scrollController = ScrollController();
  final TextEditingController cfgController = TextEditingController();

  // 防抖Timer
  Timer? _debounceTimer;
  final Duration _duration = Duration(milliseconds: 500);

  Event<String> get selectedExcelPath => _selectedExcelPath;
  Event<String> get selectedXmlFolderPath => _selectedXmlFolderPath;
  Event<String> get log => _log;
  String get cfgErrTip => _cfgErrTip;
  Event<bool> get isLoading => _isLoading;
  late SharedPreferences prefs;

  // 带防抖的配置更新方法
  void updateDefaultCfg() {
    _defaultCfg = cfgController.text;
    // 取消之前的定时器
    _debounceTimer?.cancel();
    // 设置新的定时器，500毫秒后执行保存
    _debounceTimer = Timer(_duration, () {
      _savePerference(cfgKey, _defaultCfg);
    });
  }

  void init() {
    _loadPreferences();
  }

  // 从 SharedPreferences 加载默认配置
  Future<void> _loadPreferences() async {
    prefs = await SharedPreferences.getInstance();
    final cfgCache = prefs.getString(cfgKey);
    if (cfgCache != null && cfgCache.isNotEmpty) {
      _defaultCfg = cfgCache;
    } else {
      _defaultCfg = lib.getDefaultCfg();
    }
    cfgController.text = _defaultCfg;

    // 加载之前选择的文件路径（macOS需要恢复bookmark）
    await _loadExcelPath();

    // 加载之前选择的文件夹路径（macOS需要恢复bookmark）
    await _loadXmlFolderPath();

    updateLog(
      "Cache loaded.\ncfgCache: ${cfgCache?.substring(0, cfgCache.length > 50 ? 50 : cfgCache.length) ?? 'null'}...\nexcelPath: ${_selectedExcelPath.value}\nxmlFolderPath: ${_selectedXmlFolderPath.value}",
    );
  }

  // 恢复 Excel 文件路径和访问权限
  Future<void> _loadExcelPath() async {
    if (Platform.isMacOS) {
      final bookmarkData = prefs.getString(excelBookmarkKey);
      if (bookmarkData != null && bookmarkData.isNotEmpty) {
        try {
          final resolvedFile = await _secureBookmarks.resolveBookmark(
            bookmarkData,
          );
          _selectedExcelPath.value = resolvedFile.path;
          await _secureBookmarks.startAccessingSecurityScopedResource(
            resolvedFile,
          );
          _resolvedExcelFile = resolvedFile; // 保存已解析的文件
          _updateCfgWithExcel(resolvedFile.path);
          updateLog("已恢复 Excel 文件访问权限: ${resolvedFile.path}");
          return;
        } catch (e) {
          updateLog("恢复 Excel bookmark 失败: $e");
        }
      }
    } else {
      final excelPathCache = prefs.getString(excelPathKey);
      if (excelPathCache != null && excelPathCache.isNotEmpty) {
        _selectedExcelPath.value = excelPathCache;
        _updateCfgWithExcel(excelPathCache);
      }
    }
  }

  // 恢复 XML 文件夹路径和访问权限
  Future<void> _loadXmlFolderPath() async {
    if (Platform.isMacOS) {
      final bookmarkData = prefs.getString(xmlFolderBookmarkKey);
      if (bookmarkData != null && bookmarkData.isNotEmpty) {
        try {
          final resolvedFile = await _secureBookmarks.resolveBookmark(
            bookmarkData,
            isDirectory: true,
          );
          _selectedXmlFolderPath.value = resolvedFile.path;
          await _secureBookmarks.startAccessingSecurityScopedResource(
            resolvedFile,
          );
          _resolvedXmlFolder = resolvedFile; // 保存已解析的文件夹
          updateLog("已恢复 XML 文件夹访问权限: ${resolvedFile.path}");
          return;
        } catch (e) {
          updateLog("恢复 XML folder bookmark 失败: $e");
        }
      }
    } else {
      final xmlFolderPathCache = prefs.getString(xmlFolderPathKey);
      if (xmlFolderPathCache != null && xmlFolderPathCache.isNotEmpty) {
        _selectedXmlFolderPath.value = xmlFolderPathCache;
      }
    }
  }

  // 更新缓存内容
  void _savePerference(String key, String value) {
    prefs.setString(key, value);
  }

  // 选择文件夹的方法
  Future<void> selectFolder() async {
    String? folderPath = await FilePicker.platform.getDirectoryPath();
    if (folderPath != null) {
      _selectedXmlFolderPath.value = folderPath;
      _savePerference(xmlFolderPathKey, folderPath);

      // macOS: 保存 security-scoped bookmark
      if (Platform.isMacOS) {
        try {
          final bookmark = await _secureBookmarks.bookmark(
            Directory(folderPath),
          );
          _savePerference(xmlFolderBookmarkKey, bookmark);
          updateLog("Selected XML folder: $folderPath (权限已保存)");
        } catch (e) {
          updateLog("保存文件夹 bookmark 失败: $e\nSelected XML folder: $folderPath");
        }
      } else {
        updateLog("Selected XML folder: $folderPath");
      }
    }
  }

  // 选择Excel文件的方法
  Future<void> selectExcelFile() async {
    FilePickerResult? result = await FilePicker.platform.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['xlsx', 'xls'],
    );
    if (result != null) {
      PlatformFile file = result.files.first;
      String? filePath = file.path;
      if (filePath == null) {
        updateLog("Selected file path is null.");
        return;
      }
      _selectedExcelPath.value = filePath;
      _savePerference(excelPathKey, filePath);

      // macOS: 保存 security-scoped bookmark
      if (Platform.isMacOS) {
        try {
          final bookmark = await _secureBookmarks.bookmark(File(filePath));
          _savePerference(excelBookmarkKey, bookmark);
          updateLog("Selected Excel file: $filePath (权限已保存)");
        } catch (e) {
          updateLog("保存文件 bookmark 失败: $e\nSelected Excel file: $filePath");
        }
      } else {
        updateLog("Selected Excel file: $filePath");
      }

      _updateCfgWithExcel(filePath);
    }
  }

  void _updateCfgWithExcel(String filePath) {
    try {
      final sheetNames = lib.getSheetNames(filePath: filePath);
      final json = jsonDecode(_defaultCfg);
      final sheetName = json['sheetName'];
      updateLog(sheetNames.toString());
      if (sheetName.toString().isEmpty && sheetNames.isNotEmpty) {
        // 如果 sheetName 为空且 sheetNames 不为空，则默认选择第一个 sheetName
        json['sheetName'] = sheetNames.first;
      }
      final curSheetName = json['sheetName'];
      // 检查 sheetName 是否在 sheetNames 中存在
      final isMatch = sheetNames.any(
        (element) => element == curSheetName.toString(),
      );
      if (isMatch) {
        _cfgErrTip = "";
      } else {
        _cfgErrTip = "Excel 中 没有对应的 Sheet Name";
      }
      _defaultCfg = JsonEncoder.withIndent('    ').convert(json);
      // 更新文本框内容
      cfgController.text = _defaultCfg;
    } catch (e) {
      updateLog(
        "无法读取文件: $filePath\n错误: $e\n提示: macOS 沙盒限制，缓存的路径可能无法访问，请重新选择文件",
      );
      _cfgErrTip = "文件访问失败，请重新选择";
    }
  }

  Future<void> update() async {
    final excelPath = _selectedExcelPath.value;
    final xmlFolderPath = _selectedXmlFolderPath.value;
    if (excelPath.isEmpty || xmlFolderPath.isEmpty) {
      updateLog(
        "Please select all required paths before updating. excelPath: $excelPath, xmlFolderPath: $xmlFolderPath",
      );
      return; // 确保所有路径都已选择
    }

    try {
      _isLoading.value = true;
      updateLog("开始转换...");
      final result = await lib.update(
        cfgJson: _defaultCfg,
        excelPath: excelPath,
        xmlDirPath: xmlFolderPath,
      );
      updateLog(result);
    } catch (e) {
      updateLog("转换失败: $e");
    } finally {
      _isLoading.value = false;
    }
  }

  void updateLog(String message) {
    if (_log.value.isEmpty) {
      _log.value = message;
    } else {
      _log.value += '\n$message';
    }
    // notifyListeners();
  }

  void dispose() async {
    _debounceTimer?.cancel(); // 清理防抖定时器

    // 释放 macOS security-scoped 资源
    if (Platform.isMacOS) {
      if (_resolvedExcelFile != null) {
        await _secureBookmarks.stopAccessingSecurityScopedResource(
          _resolvedExcelFile!,
        );
      }
      if (_resolvedXmlFolder != null) {
        await _secureBookmarks.stopAccessingSecurityScopedResource(
          _resolvedXmlFolder!,
        );
      }
    }

    cfgController.dispose();
    scrollController.dispose();
  }
}
