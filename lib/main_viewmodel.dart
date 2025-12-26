import 'dart:async';
import 'dart:convert';

import 'package:e2xf/event.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'src/rust/api/bridge.dart' as lib;
import 'package:shared_preferences/shared_preferences.dart';

class MainViewModel {
  static const String cfgKey = 'defaultCfg';

  MainViewModel() {
    updateLog("Application initialized.");
  }

  final Event<String> _selectedExcelPath =  Event("");
  final Event<String> _selectedXmlFolderPath = Event("");
  String _defaultCfg = "";
  final Event<String> _log = Event("");
  String _cfgErrTip = "";

  final ScrollController scrollController = ScrollController();
  final TextEditingController cfgController = TextEditingController();
  
  // 防抖Timer
  Timer? _debounceTimer;
  final Duration _duration = Duration(milliseconds: 500);

  Event<String> get selectedExcelPath => _selectedExcelPath;
  Event<String> get selectedXmlFolderPath => _selectedXmlFolderPath;
  Event<String> get log => _log;
  String get cfgErrTip => _cfgErrTip;
  late SharedPreferences prefs;

  // 带防抖的配置更新方法
  void updateDefaultCfg() {
     _defaultCfg = cfgController.text;
    // 取消之前的定时器
    _debounceTimer?.cancel();
    // 设置新的定时器，500毫秒后执行保存
    _debounceTimer = Timer(_duration, () {
      _saveDefaultCfg();
    });
  }

  void init() {
     _loadPreferences();
  }

  // 从 SharedPreferences 加载默认配置
  Future<void> _loadPreferences() async {
    prefs = await SharedPreferences.getInstance();
    final cache = prefs.getString(cfgKey);
    if (cache != null && cache.isNotEmpty) {
      updateLog("Found cached configuration in preferences.");
      _defaultCfg = cache;
    } else {
      updateLog("No cached configuration found, using library default.");
      _defaultCfg = lib.getDefaultCfg();
    }
    cfgController.text = _defaultCfg;
  }

  // 保存默认配置到 SharedPreferences
  void _saveDefaultCfg()=> prefs.setString(cfgKey, _defaultCfg);

  // 选择文件夹的方法
  Future<void> selectFolder() async {
    String? folderPath = await FilePicker.platform.getDirectoryPath();
    if (folderPath != null) {
      _selectedXmlFolderPath.value = folderPath;
      // notifyListeners();
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
      _selectedExcelPath.value = file.path;
      final sheetNames = lib.getSheetNames(filePath: _selectedExcelPath.value);
      final json = jsonDecode(_defaultCfg);
      final sheetName = json['sheetName'];
      updateLog(sheetName);
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
      updateLog(_defaultCfg);
      // notifyListeners();
    }
  }

  Future<void> update() async {
    final excelPath = _selectedExcelPath.value;
    final xmlFolderPath = _selectedXmlFolderPath.value;
    if (excelPath.isEmpty ||
        xmlFolderPath.isEmpty) {
      updateLog("Please select all required paths before updating. excelPath: $excelPath, xmlFolderPath: $xmlFolderPath");
      return; // 确保所有路径都已选择
    }
    final result = await lib.quickUpdate(
      cfgJson: _defaultCfg,
      excelPath: excelPath,
      xmlDirPath: xmlFolderPath,
    );
    updateLog(result);
  }

  void updateLog(String message) {
    if (_log.value.isEmpty) {
      _log.value = message;
    } else {
      _log.value += '\n$message';
    }
    // notifyListeners();
  }

  void dispose() {
    _debounceTimer?.cancel(); // 清理防抖定时器
    cfgController.dispose();
    scrollController.dispose();
  }
}
