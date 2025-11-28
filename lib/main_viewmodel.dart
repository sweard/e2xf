import 'dart:convert';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'src/rust/api/bridge.dart' as lib;

class MainViewModel extends ChangeNotifier {
  MainViewModel() {
    _defaultCfg = lib.getDefaultCfg();
    cfgController.text = _defaultCfg;
    updateLog("Application initialized.");
  }

  String? _selectedJsonPath;
  String? _selectedExcelPath;
  String? _selectedXmlFolderPath;
  String _defaultCfg = "";
  String _log = "";
  String _cfgErrTip = "";

  final TextEditingController cfgController = TextEditingController();

  String? get selectedJsonPath => _selectedJsonPath;
  String? get selectedExcelPath => _selectedExcelPath;
  String? get selectedXmlFolderPath => _selectedXmlFolderPath;
  String get log => _log;
  String get cfgErrTip => _cfgErrTip;

  void updateDefaultCfg() {
    _defaultCfg = cfgController.text;
  }

  // 选择文件夹的方法
  Future<void> selectFolder() async {
    String? folderPath = await FilePicker.platform.getDirectoryPath();
    if (folderPath != null) {
      _selectedXmlFolderPath = folderPath;
      notifyListeners();
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
      _selectedJsonPath = file.path;
      notifyListeners();
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
      _selectedExcelPath = file.path;
      final sheetNames = lib.getSheetNames(filePath: _selectedExcelPath!);
      final json = jsonDecode(_defaultCfg);
      final sheetName = json['sheetName'];
      updateLog(sheetName);
      updateLog(sheetNames.toString());
      if (sheetName.toString().isEmpty && sheetNames.isNotEmpty) {
        json['sheetName'] = sheetNames.first;
      }
      final curSheetName = json['sheetName'];
      final isMatch = sheetNames.any(
        (element) => element == curSheetName.toString(),
      );
      if (isMatch) {
        _cfgErrTip = "";
      } else {
        _cfgErrTip = "Excel 中 没有对应的 Sheet Name";
      }
      _defaultCfg = jsonEncode(json);
      cfgController.text = _defaultCfg;
      updateLog(_defaultCfg);
      notifyListeners();
    }
  }

  Future<void> update() async {
    if (_selectedJsonPath == null ||
        _selectedExcelPath == null ||
        _selectedXmlFolderPath == null) {
      updateLog("Please select all required paths before updating.");
      return; // 确保所有路径都已选择
    }
    final result = await lib.update(
      cfgJson: _selectedJsonPath!,
      excelPath: _selectedExcelPath!,
      xmlDirPath: _selectedXmlFolderPath!,
    );
    updateLog(result);
  }

  void updateLog(String message) {
    _log += '\n$message';
    notifyListeners();
  }

  @override
  void dispose() {
    cfgController.dispose();
    super.dispose();
  }
}
