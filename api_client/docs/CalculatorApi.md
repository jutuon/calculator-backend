# \CalculatorApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_calculator_state**](CalculatorApi.md#get_calculator_state) | **GET** /calculator_api/state | Get account's current calculator state.
[**post_calculator_state**](CalculatorApi.md#post_calculator_state) | **POST** /calculator_api/state | Update calculator state.



## get_calculator_state

> crate::models::CalculatorState get_calculator_state()
Get account's current calculator state.

Get account's current calculator state. 

### Parameters

This endpoint does not need any parameter.

### Return type

[**crate::models::CalculatorState**](CalculatorState.md)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## post_calculator_state

> post_calculator_state(calculator_state)
Update calculator state.

Update calculator state.

### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**calculator_state** | [**CalculatorState**](CalculatorState.md) |  | [required] |

### Return type

 (empty response body)

### Authorization

[api_key](../README.md#api_key)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: Not defined

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

