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
  static const int maxLogLines = 1000;
  static const String cfgKey = 'defaultCfg';
  static const String excelPathKey = 'excelPath';
  static const String xmlFolderPathKey = 'xmlFolderPath';
  static const String excelBookmarkKey = 'excelBookmark';
  static const String xmlFolderBookmarkKey = 'xmlFolderBookmark';

  final _secureBookmarks = SecureBookmarks();

  // 存储已解析的 security-scoped 资源，以便 dispose 时释放
  FileSystemEntity? _resolvedExcelFile;
  FileSystemEntity? _resolvedXmlFolder;

  // Rust 日志流订阅
  StreamSubscription<String>? _logSubscription;
  SharedPreferences? _preferences;
  Future<void>? _initialization;
  bool _disposed = false;
  final List<String> _logLines = [];

  MainViewModel() {
    updateLog("Application initialized.");
  }

  final Event<String> _selectedExcelPath = Event("");
  final Event<String> _selectedXmlFolderPath = Event("");
  String _defaultCfg = "";
  final Event<String> _log = Event("");
  final Event<String> _cfgErrTip = Event("");
  final Event<bool> _isLoading = Event(false);
  final Event<bool> _isReady = Event(false);
  final Event<bool> _useQuickUpdate = Event(true);

  final ScrollController scrollController = ScrollController();
  final TextEditingController cfgController = TextEditingController();

  // 防抖Timer
  Timer? _debounceTimer;
  final Duration _duration = Duration(milliseconds: 500);

  Event<String> get selectedExcelPath => _selectedExcelPath;
  Event<String> get selectedXmlFolderPath => _selectedXmlFolderPath;
  Event<String> get log => _log;
  String get cfgErrTip => _cfgErrTip.value;
  Event<String> get cfgErrTipEvent => _cfgErrTip;
  Event<bool> get isLoading => _isLoading;
  Event<bool> get isReady => _isReady;
  Event<bool> get useQuickUpdate => _useQuickUpdate;

  // 带防抖的配置更新方法
  void updateDefaultCfg() {
    _defaultCfg = cfgController.text;
    // 取消之前的定时器
    _debounceTimer?.cancel();
    // 设置新的定时器，500毫秒后执行保存
    _debounceTimer = Timer(_duration, () {
      unawaited(_savePreference(cfgKey, _defaultCfg));
    });
  }

  Future<void> init() {
    return _initialization ??= _initialize();
  }

  Future<void> _initialize() async {
    _logSubscription = lib.createLogStream().listen((message) {
      updateLog(message);
    });
    try {
      await _loadPreferences();
    } catch (e) {
      if (_disposed) {
        return;
      }
      updateLog("初始化失败: $e");
      if (_defaultCfg.isEmpty) {
        _defaultCfg = lib.getDefaultCfg();
        cfgController.text = _defaultCfg;
      }
    } finally {
      if (!_disposed) {
        _isReady.value = true;
      }
    }
  }

  // 从 SharedPreferences 加载默认配置
  Future<void> _loadPreferences() async {
    final preferences = await SharedPreferences.getInstance();
    if (_disposed) {
      return;
    }
    _preferences = preferences;
    final cfgCache = preferences.getString(cfgKey);
    if (cfgCache != null && cfgCache.isNotEmpty) {
      _defaultCfg = cfgCache;
    } else {
      _defaultCfg = lib.getDefaultCfg();
    }
    cfgController.text = _defaultCfg;

    // 加载之前选择的文件路径（macOS需要恢复bookmark）
    await _loadExcelPath();
    if (_disposed) {
      return;
    }

    // 加载之前选择的文件夹路径（macOS需要恢复bookmark）
    await _loadXmlFolderPath();

    updateLog(
      "Cache loaded.\ncfgCache: ${cfgCache?.substring(0, cfgCache.length > 50 ? 50 : cfgCache.length) ?? 'null'}...\nexcelPath: ${_selectedExcelPath.value}\nxmlFolderPath: ${_selectedXmlFolderPath.value}",
    );
  }

  // 恢复 Excel 文件路径和访问权限
  Future<void> _loadExcelPath() async {
    final preferences = _preferences;
    if (_disposed || preferences == null) {
      return;
    }
    if (Platform.isMacOS) {
      final bookmarkData = preferences.getString(excelBookmarkKey);
      if (bookmarkData != null && bookmarkData.isNotEmpty) {
        try {
          final resolvedFile = await _secureBookmarks.resolveBookmark(
            bookmarkData,
          );
          if (_disposed) {
            return;
          }
          _selectedExcelPath.value = resolvedFile.path;
          await _secureBookmarks.startAccessingSecurityScopedResource(
            resolvedFile,
          );
          if (_disposed) {
            await _secureBookmarks.stopAccessingSecurityScopedResource(
              resolvedFile,
            );
            return;
          }
          _resolvedExcelFile = resolvedFile; // 保存已解析的文件
          _updateCfgWithExcel(resolvedFile.path);
          updateLog("已恢复 Excel 文件访问权限: ${resolvedFile.path}");
          return;
        } catch (e) {
          updateLog("恢复 Excel bookmark 失败: $e");
        }
      }
    } else {
      final excelPathCache = preferences.getString(excelPathKey);
      if (excelPathCache != null && excelPathCache.isNotEmpty) {
        _selectedExcelPath.value = excelPathCache;
        _updateCfgWithExcel(excelPathCache);
      }
    }
  }

  // 恢复 XML 文件夹路径和访问权限
  Future<void> _loadXmlFolderPath() async {
    final preferences = _preferences;
    if (_disposed || preferences == null) {
      return;
    }
    if (Platform.isMacOS) {
      final bookmarkData = preferences.getString(xmlFolderBookmarkKey);
      if (bookmarkData != null && bookmarkData.isNotEmpty) {
        try {
          final resolvedFile = await _secureBookmarks.resolveBookmark(
            bookmarkData,
            isDirectory: true,
          );
          if (_disposed) {
            return;
          }
          _selectedXmlFolderPath.value = resolvedFile.path;
          await _secureBookmarks.startAccessingSecurityScopedResource(
            resolvedFile,
          );
          if (_disposed) {
            await _secureBookmarks.stopAccessingSecurityScopedResource(
              resolvedFile,
            );
            return;
          }
          _resolvedXmlFolder = resolvedFile; // 保存已解析的文件夹
          updateLog("已恢复 XML 文件夹访问权限: ${resolvedFile.path}");
          return;
        } catch (e) {
          updateLog("恢复 XML folder bookmark 失败: $e");
        }
      }
    } else {
      final xmlFolderPathCache = preferences.getString(xmlFolderPathKey);
      if (xmlFolderPathCache != null && xmlFolderPathCache.isNotEmpty) {
        _selectedXmlFolderPath.value = xmlFolderPathCache;
      }
    }
  }

  // 更新缓存内容
  Future<void> _savePreference(String key, String value) async {
    final preferences = _preferences;
    if (preferences == null) {
      updateLog("偏好设置尚未就绪，未保存: $key");
      return;
    }
    await preferences.setString(key, value);
  }

  // 选择文件夹的方法
  Future<void> selectFolder() async {
    String? folderPath = await FilePicker.platform.getDirectoryPath();
    if (_disposed) {
      return;
    }
    if (folderPath != null) {
      _selectedXmlFolderPath.value = folderPath;
      await _savePreference(xmlFolderPathKey, folderPath);

      // macOS: 保存 security-scoped bookmark
      if (Platform.isMacOS) {
        try {
          final bookmark = await _secureBookmarks.bookmark(
            Directory(folderPath),
          );
          await _savePreference(xmlFolderBookmarkKey, bookmark);
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
    if (_disposed) {
      return;
    }
    if (result != null) {
      PlatformFile file = result.files.first;
      String? filePath = file.path;
      if (filePath == null) {
        updateLog("Selected file path is null.");
        return;
      }
      _selectedExcelPath.value = filePath;
      await _savePreference(excelPathKey, filePath);

      // macOS: 保存 security-scoped bookmark
      if (Platform.isMacOS) {
        try {
          final bookmark = await _secureBookmarks.bookmark(File(filePath));
          await _savePreference(excelBookmarkKey, bookmark);
          updateLog("Selected Excel file: $filePath (权限已保存)");
        } catch (e) {
          updateLog("保存文件 bookmark 失败: $e\nSelected Excel file: $filePath");
        }
      } else {
        updateLog("Selected Excel file: $filePath");
      }

      if (_disposed) {
        return;
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
        _cfgErrTip.value = "";
      } else {
        _cfgErrTip.value = "Excel 中 没有对应的 Sheet Name";
      }
      _defaultCfg = JsonEncoder.withIndent('    ').convert(json);
      // 更新文本框内容
      cfgController.text = _defaultCfg;
    } catch (e) {
      updateLog(
        "无法读取文件: $filePath\n错误: $e\n提示: macOS 沙盒限制，缓存的路径可能无法访问，请重新选择文件",
      );
      _cfgErrTip.value = "文件访问失败，请重新选择";
    }
  }

  void toggleUseQuickUpdate(bool value) {
    _useQuickUpdate.value = value;
  }

  Future<void> update() async {
    if (_disposed || !_isReady.value) {
      return;
    }
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
      if (useQuickUpdate.value) {
        await lib.quickUpdate(
          cfgJson: _defaultCfg,
          excelPath: excelPath,
          xmlDirPath: xmlFolderPath,
        );
      } else {
        await lib.update(
          cfgJson: _defaultCfg,
          excelPath: excelPath,
          xmlDirPath: xmlFolderPath,
        );
      }
      updateLog("转换成功");
    } catch (e) {
      updateLog("转换失败: $e");
    } finally {
      if (!_disposed) {
        _isLoading.value = false;
      }
    }
  }

  /// 切换 Rust 端日志级别: true = Debug,false = Info(默认)。
  void setLogDebug(bool enable) {
    lib.setLogDebug(enable: enable);
  }

  void updateLog(String message) {
    if (_disposed || message.isEmpty) {
      return;
    }
    _logLines.addAll(const LineSplitter().convert(message));
    if (_logLines.length > maxLogLines) {
      _logLines.removeRange(0, _logLines.length - maxLogLines);
    }
    _log.value = _logLines.join('\n');
  }

  void dispose() {
    if (_disposed) {
      return;
    }
    _disposed = true;
    _debounceTimer?.cancel(); // 清理防抖定时器
    unawaited(_releaseAsyncResources());

    cfgController.dispose();
    scrollController.dispose();
    _selectedExcelPath.dispose();
    _selectedXmlFolderPath.dispose();
    _log.dispose();
    _cfgErrTip.dispose();
    _isLoading.dispose();
    _isReady.dispose();
    _useQuickUpdate.dispose();
  }

  Future<void> _releaseAsyncResources() async {
    try {
      await _logSubscription?.cancel(); // 取消日志流订阅
    } catch (_) {
      // 应用正在退出，清理失败不再影响界面状态。
    }

    // 释放 macOS security-scoped 资源
    if (Platform.isMacOS) {
      if (_resolvedExcelFile != null) {
        try {
          await _secureBookmarks.stopAccessingSecurityScopedResource(
            _resolvedExcelFile!,
          );
        } catch (_) {
          // 应用正在退出，系统也会回收 security-scoped 访问。
        }
      }
      if (_resolvedXmlFolder != null) {
        try {
          await _secureBookmarks.stopAccessingSecurityScopedResource(
            _resolvedXmlFolder!,
          );
        } catch (_) {
          // 应用正在退出，系统也会回收 security-scoped 访问。
        }
      }
    }
  }
}
