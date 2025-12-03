import 'dart:convert';

import 'package:e2xf/event.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'src/rust/api/bridge.dart' as lib;

class MainViewModel {
  MainViewModel() {
    _defaultCfg = lib.getDefaultCfg();
    cfgController.text = _defaultCfg;
    updateLog("Application initialized.");
  }

  final Event<String> _selectedJsonPath = Event("");
  final Event<String> _selectedExcelPath =  Event("");
  final Event<String> _selectedXmlFolderPath = Event("");
  String _defaultCfg = "";
  final Event<String> _log = Event("");
  String _cfgErrTip = "";


  final TextEditingController cfgController = TextEditingController();

  Event<String> get selectedJsonPath => _selectedJsonPath;
  Event<String> get selectedExcelPath => _selectedExcelPath;
  Event<String> get selectedXmlFolderPath => _selectedXmlFolderPath;
  Event<String> get log => _log;
  String get cfgErrTip => _cfgErrTip;

  void updateDefaultCfg() {
    _defaultCfg = cfgController.text;
  }

  // 选择文件夹的方法
  Future<void> selectFolder() async {
    String? folderPath = await FilePicker.platform.getDirectoryPath();
    if (folderPath != null) {
      _selectedXmlFolderPath.value = folderPath;
      // notifyListeners();
    }
  }

  // 选择JSON文件的方法
  Future<void> selectJsonFile() async {
    FilePickerResult? result = await FilePicker.platform.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['json'],
    );
    if (result != null) {
      PlatformFile file = result.files.first;
      _selectedJsonPath.value = file.path;
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
      _defaultCfg = jsonEncode(json);
      // 更新文本框内容
      cfgController.text = _defaultCfg;
      updateLog(_defaultCfg);
      // notifyListeners();
    }
  }

  Future<void> update() async {
    if (_selectedJsonPath.value.isEmpty ||
        _selectedExcelPath.value.isEmpty ||
        _selectedXmlFolderPath.value.isEmpty) {
      updateLog("Please select all required paths before updating.");
      return; // 确保所有路径都已选择
    }
    final result = await lib.update(
      cfgJson: _selectedJsonPath.value,
      excelPath: _selectedExcelPath.value,
      xmlDirPath: _selectedXmlFolderPath.value,
    );
    updateLog(result);
  }

  void updateLog(String message) {
    _log.value += '\n$message';
    // notifyListeners();
  }
}
